//! The browser side: components, state, routing and local storage.

use active::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, PartialEq)]
struct Todo {
    id: u32,
    text: String,
    done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Done,
}

#[wasm_bindgen(start)]
pub fn start() {
    active::mount_to("#app", app());
}

fn app() -> Node {
    let path = location();
    // Created here, above the router: the list survives page changes.
    let todos = use_state(load());
    effect(move || todos.with(|list| save(list)));

    let link = move |href: &'static str, label: &'static str| {
        a().href(href)
            .class_if("on", move || path.get() == href)
            .text(label)
    };
    div()
        .class("app")
        .child(super::site_seo())
        .child(
            nav()
                .child(link("/", "To do"))
                .child(link("/about", "About")),
        )
        .child(main().children_dyn(move || match path.get().as_str() {
            "/" => vec![todo_page(todos)],
            "/about" => vec![about()],
            _ => vec![not_found()],
        }))
        .into()
}

/// The filter is in the URL (`/?show=done`), so it survives a reload and can be shared.
fn filter() -> Filter {
    match query_param("show").as_deref() {
        Some("active") => Filter::Active,
        Some("done") => Filter::Done,
        _ => Filter::All,
    }
}

fn todo_page(todos: State<Vec<Todo>>) -> Node {
    let draft = use_state(String::new());

    let add = move |e: Event| {
        e.prevent_default();
        let text = draft.get().trim().to_string();
        if text.is_empty() {
            return;
        }
        todos.update(|list| {
            let id = list.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            list.push(Todo {
                id,
                text,
                done: false,
            });
        });
        draft.set(String::new());
    };
    let filter_link = |value: Filter, href: &'static str, label: &'static str| {
        a().href(href)
            .class_if("on", move || filter() == value)
            .text(label)
    };

    section()
        .child(Seo::new().title("To do"))
        .child(h1().text("To do"))
        .child(
            form()
                .on("submit", add)
                .child(
                    input()
                        .attr("type", "text")
                        .attr("aria-label", "New to-do")
                        .attr("placeholder", "What needs doing?")
                        .attr_dyn("value", move || draft.get())
                        .on_input(move |e| draft.set(e.value())),
                )
                .child(button().text("Add")),
        )
        // Kept by id: adding, checking or removing a to-do rebuilds only that row.
        .child(ul().children_keyed(
            move || {
                let shown = filter();
                todos.with(|list| {
                    list.iter()
                        .filter(|t| match shown {
                            Filter::All => true,
                            Filter::Active => !t.done,
                            Filter::Done => t.done,
                        })
                        .cloned()
                        .collect()
                })
            },
            |todo| todo.id,
            move |todo| item(todos, todo.clone()),
        ))
        .child(
            div()
                .class("bar")
                .child(span().class("left").text_dyn(move || {
                    let left = todos.with(|l| l.iter().filter(|t| !t.done).count());
                    format!("{left} left")
                }))
                .child(filter_link(Filter::All, "/", "All"))
                .child(filter_link(Filter::Active, "/?show=active", "Active"))
                .child(filter_link(Filter::Done, "/?show=done", "Done")),
        )
        .into()
}

fn item(todos: State<Vec<Todo>>, todo: Todo) -> Node {
    let id = todo.id;
    let checkbox = input().attr("type", "checkbox").on("change", move |_| {
        todos.update(|l| {
            l.iter_mut()
                .filter(|t| t.id == id)
                .for_each(|t| t.done = !t.done)
        })
    });
    li().class(if todo.done { "done" } else { "" })
        .child(if todo.done {
            checkbox.attr("checked", "")
        } else {
            checkbox
        })
        .child(span().text(todo.text))
        .child(
            button()
                .attr("aria-label", "Delete")
                .on_click(move |_| todos.update(|l| l.retain(|t| t.id != id)))
                .text("×"),
        )
        .into()
}

fn about() -> Node {
    section()
        .child(
            Seo::new()
                .title("About")
                .description("How this to-do list is built."),
        )
        .child(h1().text("About"))
        .child(p().text(
            "This app is Rust compiled to WebAssembly. The server sends one empty page; \
             the browser renders the views, follows the links without reloading and \
             updates the title and the meta tags of each page.",
        ))
        .child(p().child(a().href("/").text("Back to the list")))
        .into()
}

fn not_found() -> Node {
    section()
        .child(Seo::new().title("Not found").noindex(true))
        .child(h1().text("Not found"))
        .child(p().child(a().href("/").text("Back to the list")))
        .into()
}

const KEY: &str = "active-todos";

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// One to-do per line: `0` or `1`, a tab, the text.
fn load() -> Vec<Todo> {
    let saved = storage()
        .and_then(|s| s.get_item(KEY).ok().flatten())
        .unwrap_or_default();
    saved
        .lines()
        .zip(1..)
        .filter_map(|(line, id)| {
            let (done, text) = line.split_once('\t')?;
            Some(Todo {
                id,
                text: text.to_string(),
                done: done == "1",
            })
        })
        .collect()
}

fn save(list: &[Todo]) {
    let lines: Vec<String> = list
        .iter()
        .map(|t| {
            format!(
                "{}\t{}",
                u8::from(t.done),
                t.text.replace(['\n', '\t'], " ")
            )
        })
        .collect();
    if let Some(s) = storage() {
        let _ = s.set_item(KEY, &lines.join("\n"));
    }
}
