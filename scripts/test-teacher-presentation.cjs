// Actual app rendering and presentation facts; synthetic fixture, no model calls.
const { chromium } = require(process.env.RUFREN_PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const { navigate } = require('./qa-navigation.cjs');
const response = {
  request: { mode: 'answer_feedback', input: {
    request_id: 'presentation-fixture', revision_id: 'fixture-revision',
    attempt_id: null, context_id: null, exercise_id: null,
    skill: 'writing', task: 'Synthetic presentation test',
    prompt: 'Say that you are reading.', reference: null,
    response: 'Я читать.', cues: [], evidence: 'written_text',
  } },
  provenance: { provider: 'qa_synthetic', model: 'fixture',
    review_status: 'ai_generated_teaching', scope: 'Synthetic UI fixture; no model evaluation',
    prompt_version: 'fixture-v1', response_id: 'presentation-response-1' },
  usage: { accounting: 'subscription_quota', input_tokens: 0, output_tokens: 0,
    estimated_micro_usd: 0, reserved_micro_usd: 0 },
  content: { explanation: 'A synthetic correction for rendering checks.', examples: [],
    practice: [], next_step: 'No learner task is assigned by this fixture.',
    caveats: ['**Ёжик.** A separate collapsed fixture block.'],
    feedback: { verdict: 'partly_correct', suggested_response: 'Я читаю книгу.',
      corrections: [{ original: 'Я читать.', replacement: 'Я читаю.',
        reason: 'Use the first-person form.' }] } },
};
(async () => {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1000, height: 700 } });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const seen = async text => page.evaluate(text => window.rufrenLibrary.entries()
    .filter(([key]) => key.startsWith('rufren.v1.event.'))
    .map(([,value]) => JSON.parse(value))
    .some(event => event.fact.kind === 'stimulus_presented' && event.fact.data.russian === text), text);
  const waitSeen = text => page.waitForFunction(text => window.rufrenLibrary.entries()
    .filter(([key]) => key.startsWith('rufren.v1.event.'))
    .map(([,value]) => JSON.parse(value))
    .some(event => event.fact.kind === 'stimulus_presented' && event.fact.data.russian === text), text);
  try {
    await page.goto(process.env.RUFREN_QA_URL || 'http://127.0.0.1:8091');
    await page.getByRole('navigation', { name: 'Main navigation sidebar', exact: true }).waitFor();
    await page.evaluate(value => window.rufrenLibrary.write('rufren.v1.meta.teacher_response', JSON.stringify(value)), response);
    await navigate(page, 'Teacher');
    await page.locator('.teacher-correction').scrollIntoViewIfNeeded();
    await waitSeen('Я читать. → Я читаю.');
    await waitSeen('Я читаю книгу.');
    assert.equal(await seen('Ёжик. A separate collapsed fixture block.'), false);
    const nuance = page.getByText('Nuance and uncertainty', { exact: true });
    await nuance.click();
    await page.getByText('A separate collapsed fixture block.', { exact: false }).scrollIntoViewIfNeeded();
    await waitSeen('Ёжик. A separate collapsed fixture block.');
    assert.deepEqual(errors, []);
    console.log('PASS actual correction/suggestion exposure; collapsed caveat withheld, rich visible caveat acknowledged');
  } finally {
    await context.close();
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
