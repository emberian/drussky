//! Recover one whole local editing window; never copy an answered practice into another tab.
use dioxus::prelude::*;

#[component]
pub(crate) fn SavedWindows() -> Element {
    #[cfg(not(target_arch = "wasm32"))]
    {
        rsx! {}
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut windows = use_resource(crate::repository::browser_workspaces);
        let mut busy = use_signal(|| false);
        let mut notice = use_signal(String::new);
        let snapshot = windows.read().clone();
        rsx! {
            section {class:"panel",h2 {"Your saved windows"}
                p {"Each tab keeps its own practice and passage drafts. Reopen an earlier window here after closing it elsewhere."}
                button {class:"text-button",disabled:busy(),onclick:move |_|windows.restart(),"Refresh saved windows"}
                if !notice().is_empty() {p {role:"alert","{notice}"}}
                match snapshot {
                    None=>rsx!{p {class:"quiet","Finding your saved work…"}},
                    Some(Err(error))=>rsx!{p {role:"alert","{error}"}},
                    Some(Ok(entries))=>rsx!{
                        for entry in entries {
                            {let id=entry.id.clone();
                            let title=entry.passage_title.filter(|title|!title.trim().is_empty()).unwrap_or_else(||if entry.practice_response.is_some(){"Saved practice".into()}else{"Saved window".into()});
                            let preview=entry.practice_response.unwrap_or_default().chars().take(120).collect::<String>();
                            rsx!{div {class:"history-row",
                                div {strong {"{title}"}small {"Last used: {entry.last_used_label}"}
                                    if !preview.is_empty(){p {class:"quiet","{preview}"}}
                                    p {class:"quiet","{entry.passage_count} passage drafts · {entry.paused_count} paused practices"}
                                }
                                button {class:"secondary",disabled:busy()||entry.current||entry.active,onclick:move |_|{
                                    busy.set(true);notice.set(String::new());let id=id.clone();
                                    spawn(async move {if let Err(error)=crate::repository::switch_browser_workspace(&id).await{notice.set(error);busy.set(false);}});
                                },if entry.current {"This window"}else if entry.active {"Open in another tab"}else{"Reopen here"}}
                            }}}
                        }
                    },
                }
            }
        }
    }
}
