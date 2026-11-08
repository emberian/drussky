"""A small ONNX interpreter in PyTorch, used by models.py to trace an ONNX graph into TorchScript
for coremltools (which no longer reads ONNX itself).

Values derived only from constants and shapes are folded in NumPy, so `If`, `Range` and `Shape`
chains disappear; values that depend on the inputs are torch tensors and get traced. The graph is
traced at one padded length T: shape arguments that would depend on the true length (the
`x_lens` input) take the value they have at length T (`prepare`), while the padding mask stays a
runtime computation from `x_lens`. Only the ops the ZIPA CTC export uses are implemented.
"""
import numpy as np, torch, torch.nn.functional as F
import onnx
from onnx import numpy_helper

TDT = {1: torch.float32, 6: torch.int32, 7: torch.int64, 9: torch.bool, 10: torch.float16, 11: torch.float64}


def attrs(n):
    out = {}
    for a in n.attribute:
        v = onnx.helper.get_attribute_value(a)
        if isinstance(v, bytes): v = v.decode()
        if a.type == onnx.AttributeProto.TENSOR: v = numpy_helper.to_array(v)
        out[a.name] = v
    return out


STATIC_ARGS = {"ReduceMean": [1], "ReduceSum": [1], "ReduceMax": [1], "Unsqueeze": [1], "Squeeze": [1], "Reshape": [1],
               "Slice": [1, 2, 3, 4], "Expand": [1], "Tile": [1], "Range": [0, 1, 2], "ConstantOfShape": [0], "ScatterND": [1]}


def is_static(v): return isinstance(v, np.ndarray) or v is None


def T(v):
    return v if isinstance(v, torch.Tensor) else torch.from_numpy(np.array(v, copy=True))


class Runner(torch.nn.Module):
    def __init__(self, model):
        super().__init__()
        self.graph = model.graph
        self.params = torch.nn.ParameterDict()
        self.init = {}
        self.shadow = None      # value name -> static value seen when x_lens == padded length
        self.recording = None
        for i, t in enumerate(self.graph.initializer):
            a = numpy_helper.to_array(t)
            if a.dtype == np.float32 and a.size > 16:
                key = f"p{i}"
                self.params[key] = torch.nn.Parameter(torch.from_numpy(a.copy()), requires_grad=False)
                self.init[t.name] = key
            else:
                self.init[t.name] = a.copy()

    def forward(self, *inputs):
        env = {}
        for k, v in self.init.items():
            env[k] = self.params[v] if isinstance(v, str) else v
        for i, x in zip(self.graph.input, inputs): env[i.name] = x
        self.run(self.graph, env)
        return tuple(env[o.name] for o in self.graph.output)

    def run(self, g, env):
        prod = {o: n for n in g.node for o in n.output}
        self.prod = getattr(self, "prod", {}); self.prod.update(prod)
        for t in g.initializer:
            env.setdefault(t.name, numpy_helper.to_array(t).copy())
        for n in g.node:
            ins = [env[i] if i else None for i in n.input]
            self.cur = (n.name, [type(v).__name__ + str(getattr(v, "shape", "")) for v in ins])
            try:
                outs = self.op(n, ins, attrs(n), env)
            except Exception as e:
                raise RuntimeError(f"ONNX node {n.op_type} {self.cur}") from e
            if not isinstance(outs, (list, tuple)): outs = [outs]
            for name, v in zip(n.output, outs): env[name] = v
            if self.recording is not None:
                for name, v in zip(n.output, outs):
                    if isinstance(v, np.ndarray) and v.size <= 64:
                        self.recording[name] = v.copy()

    def op(self, n, x, a, env):
        o = n.op_type
        for i in STATIC_ARGS.get(o, []):
            if i < len(x) and isinstance(x[i], torch.Tensor) and self.shadow is not None:
                x = list(x); x[i] = self.shadow[n.input[i]]
        st = all(is_static(v) for v in x)
        if o == "Constant": return a["value"]
        if o == "Log" and not st:
            # log(1 + exp(z)) is softplus; written out it overflows in float16 on the Neural Engine.
            add = self.prod.get(n.input[0])
            if add is not None and add.op_type == "Add":
                for i, j in ((0, 1), (1, 0)):
                    e, c = self.prod.get(add.input[i]), env.get(add.input[j])
                    if e is not None and e.op_type == "Exp" and is_static(c) and c is not None and np.allclose(c, 1):
                        return F.softplus(T(env[e.input[0]]))
        if o == "Shape": return np.array([int(d) for d in x[0].shape], dtype=np.int64)
        if o == "If":
            assert is_static(x[0]), n.name
            br = a["then_branch"] if bool(np.asarray(x[0]).reshape(-1)[0]) else a["else_branch"]
            sub = dict(env); self.run(br, sub)
            return [sub[out.name] for out in br.output]
        if st:
            return self.np_op(o, x, a)
        return self.torch_op(o, [T(v) if v is not None else None for v in x], a, x)

    def np_op(self, o, x, a):
        # Fold with torch on CPU for exact semantics, then back to numpy.
        r = self.torch_op(o, [torch.from_numpy(np.array(v, copy=True)) if v is not None else None for v in x], a, x)
        if isinstance(r, (list, tuple)): return [t.numpy() for t in r]
        return r.numpy()

    def torch_op(self, o, x, a, raw):
        for i in STATIC_ARGS.get(o, []):
            if i < len(raw) and isinstance(raw[i], torch.Tensor):
                raise RuntimeError(f"dynamic shape argument {i} to {o} at {self.cur}")
        if o == "Add": return x[0] + x[1]
        if o == "Sub": return x[0] - x[1]
        if o == "Mul": return x[0] * x[1]
        if o == "Div":
            if not x[0].is_floating_point() and not x[1].is_floating_point(): return torch.div(x[0], x[1], rounding_mode="trunc")
            return x[0] / x[1]
        if o == "Pow": return torch.pow(x[0], x[1])
        if o == "Neg": return -x[0]
        if o == "Abs": return torch.abs(x[0])
        if o == "Exp": return torch.exp(x[0])
        if o == "Log": return torch.log(x[0])
        if o == "Sigmoid": return torch.sigmoid(x[0])
        if o == "Tanh": return torch.tanh(x[0])
        if o == "Sign": return torch.sign(x[0])
        if o == "Atan": return torch.atan(x[0])
        if o == "Cos": return torch.cos(x[0])
        if o == "Sin": return torch.sin(x[0])
        if o == "Max":
            r = x[0]
            for y in x[1:]: r = torch.maximum(r, y)
            return r
        if o == "Equal": return x[0] == x[1]
        if o == "GreaterOrEqual": return x[0] >= x[1]
        if o == "Less": return x[0] < x[1]
        if o == "Greater": return x[0] > x[1]
        if o == "Not": return ~x[0]
        if o == "Where": return torch.where(x[0], x[1], x[2])
        if o == "MatMul": return torch.matmul(x[0], x[1])
        if o == "Cast": return x[0].to(TDT[a["to"]])
        if o == "Softmax": return torch.softmax(x[0], dim=a.get("axis", -1))
        if o == "LogSoftmax": return torch.log_softmax(x[0], dim=a.get("axis", -1))
        if o in ("ReduceMean", "ReduceSum", "ReduceMax"):
            axes = a.get("axes")
            if axes is None and len(x) > 1 and x[1] is not None: axes = [int(v) for v in raw[1]]
            keep = bool(a.get("keepdims", 1))
            if axes is None: axes = list(range(x[0].dim()))
            axes = [int(v) for v in axes]
            if o == "ReduceMean": return x[0].mean(dim=axes, keepdim=keep)
            if o == "ReduceSum": return x[0].sum(dim=axes, keepdim=keep)
            r = x[0]
            for ax in sorted([d % r.dim() for d in axes], reverse=True): r = r.amax(dim=ax, keepdim=keep)
            return r
        if o == "Unsqueeze":
            axes = a.get("axes")
            if axes is None: axes = [int(v) for v in raw[1]]
            r = x[0]
            nd = r.dim() + len(axes)
            for ax in sorted(d % nd for d in axes): r = r.unsqueeze(ax)
            return r
        if o == "Squeeze":
            axes = a.get("axes")
            if axes is None and len(x) > 1: axes = [int(v) for v in raw[1]]
            r = x[0]
            if axes is None: return r.squeeze()
            for ax in sorted((d % r.dim() for d in axes), reverse=True): r = r.squeeze(ax)
            return r
        if o == "Concat":
            keep = [v for v in x if v.numel() > 0] or x[:1]
            return keep[0] if len(keep) == 1 else torch.cat(keep, dim=a["axis"])
        if o == "Reshape":
            if not is_static(raw[1]): raise RuntimeError(f"dynamic reshape {self.cur}")
            shape = [int(v) for v in np.asarray(raw[1]).reshape(-1)]
            shape = [int(x[0].shape[i]) if s == 0 else s for i, s in enumerate(shape)]
            return x[0].reshape(shape)
        if o == "Transpose": return x[0].permute(*a["perm"])
        if o in ("Gather", "Slice", "GatherElements", "Tile") and x[0] is not None and x[0].dtype == torch.bool:
            return self.torch_op(o, [x[0].to(torch.int32)] + x[1:], a, raw) != 0
        if o == "Gather":
            ax = a.get("axis", 0)
            idx = x[1]
            if is_static(raw[1]):
                idx_np = np.asarray(raw[1]).astype(np.int64)
                idx_np = np.where(idx_np < 0, idx_np + int(x[0].shape[ax]), idx_np)
                if idx_np.ndim == 0: return x[0].select(ax, int(idx_np))
                return torch.index_select(x[0], ax, torch.from_numpy(idx_np.reshape(-1))).reshape(
                    list(x[0].shape[:ax]) + list(idx_np.shape) + list(x[0].shape[ax + 1:]))
            idx = torch.where(idx < 0, idx + int(x[0].shape[ax]), idx)
            return torch.index_select(x[0], ax, idx.reshape(-1)).reshape(list(x[0].shape[:ax]) + list(idx.shape) + list(x[0].shape[ax + 1:]))
        if o == "GatherElements": return torch.gather(x[0], a.get("axis", 0), x[1].long())
        if o == "Slice":
            starts, ends = [int(v) for v in raw[1]], [int(v) for v in raw[2]]
            axes = [int(v) for v in raw[3]] if len(raw) > 3 and raw[3] is not None else list(range(len(starts)))
            steps = [int(v) for v in raw[4]] if len(raw) > 4 and raw[4] is not None else [1] * len(starts)
            r = x[0]
            for s, e, ax, stp in zip(starts, ends, axes, steps):
                dim = int(r.shape[ax])
                if stp > 0:
                    s = max(0, min(dim, s + dim if s < 0 else s)); e = max(0, min(dim, e + dim if e < 0 else e))
                    r = r.narrow(ax, s, max(0, e - s)) if stp == 1 else r.index_select(ax, torch.arange(s, e, stp))
                else:
                    s = max(-1, min(dim - 1, s + dim if s < 0 else s)); e = max(-1, min(dim - 1, e + dim if e < 0 else e))
                    r = r.index_select(ax, torch.arange(s, e, stp))
            return r
        if o == "Expand":
            shape = tuple(int(v) for v in raw[1])
            return x[0].expand(torch.broadcast_shapes(tuple(x[0].shape), shape))
        if o == "Tile": return x[0].repeat(*[int(v) for v in raw[1]])
        if o == "Range":
            return torch.arange(raw[0].item(), raw[1].item(), raw[2].item(), dtype=x[0].dtype)
        if o == "ConstantOfShape":
            v = a.get("value", np.zeros(1, np.float32))
            return torch.full([int(s) for s in raw[0]], v.reshape(-1)[0].item(), dtype=torch.from_numpy(v).dtype)
        if o == "ScatterND":
            data, idx, upd = x[0].clone(), raw[1], x[2]
            idx = np.asarray(idx)
            flat = idx.reshape(-1, idx.shape[-1])
            u = upd.reshape(-1, *upd.shape[idx.ndim - 1:])
            for k, ix in enumerate(flat): data[tuple(int(i) for i in ix)] = u[k]
            return data
        if o == "Conv":
            w, b = x[1], x[2] if len(x) > 2 else None
            nd = w.dim() - 2
            pads = a.get("pads", [0] * (2 * nd))
            stride, dil, grp = a.get("strides", [1] * nd), a.get("dilations", [1] * nd), a.get("group", 1)
            sym = all(pads[i] == pads[i + nd] for i in range(nd))
            inp = x[0]
            if not sym:
                pp = []
                for i in reversed(range(nd)): pp += [pads[i], pads[i + nd]]
                inp = F.pad(inp, pp); pad = 0
            else:
                pad = tuple(pads[:nd])
            f = F.conv1d if nd == 1 else F.conv2d
            return f(inp, w, b, stride=stride, padding=pad, dilation=dil, groups=grp)
        raise NotImplementedError(o)


def prepare(runner, T):
    """Record the static max-length of every ReduceMax at full length T, so masks are built
    against the padded size while the true lengths stay a runtime input."""
    runner.recording = {}
    with torch.no_grad():
        runner(torch.zeros(1, T, 80), np.array([T], dtype=np.int64))
    runner.shadow, runner.recording = runner.recording, None
    return runner
