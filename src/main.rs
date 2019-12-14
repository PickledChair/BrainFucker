extern crate iui;
extern crate rusty_brainfuck;

use iui::prelude::*;
use iui::controls::{VerticalBox, HorizontalBox, LayoutStrategy, Button, Label, Entry, MultilineEntry};
use iui::menus::Menu;

use rusty_brainfuck::Brainfuck;

use futures::{
    executor::ThreadPool,
    future::{Future, AbortHandle, Aborted, abortable},
    task::SpawnExt,
    channel::mpsc::{channel, Sender},
};

use std::fs;
use std::io::Write;
use std::rc::Rc;
use std::cell::RefCell;

struct AppState<F: Future<Output = Result<(), Aborted>>> {
    bf_futures: Vec<F>,
    running: bool,
    stop: bool,
    input_exists: bool,
}

fn main() {
    let ui = UI::init().expect("Couldn't initialize UI library");

    let app_state = Rc::new(RefCell::new(
        AppState {
            bf_futures: Vec::new(),
            running: false,
            stop: false,
            input_exists: false,
        }
    ));

    let mut vertbox = VerticalBox::new(&ui);
    vertbox.set_padded(&ui, true);
    let mut run_horibox = HorizontalBox::new(&ui);
    run_horibox.set_padded(&ui, true);
    let mut input_horibox = HorizontalBox::new(&ui);
    input_horibox.set_padded(&ui, true);

    let (source_label,
         source_multi,
         mut run_button,
         mut stop_button,
         result_multi,
         input_entry,
         mut input_button
    ) = {
        let source_label = Label::new(&ui, "Source:");
        let source_multi = MultilineEntry::new(&ui);

        let mut run_horibox = run_horibox.clone();
        let run_button = Button::new(&ui, "Run");
        let stop_button = Button::new(&ui, "Stop");
        run_horibox.append(&ui, run_button.clone(), LayoutStrategy::Compact);
        run_horibox.append(&ui, stop_button.clone(), LayoutStrategy::Compact);

        let result_label = Label::new(&ui, "Result:");
        let mut result_multi = MultilineEntry::new(&ui);
        result_multi.set_value(&ui, "[Brainfuck Interpreter is ready]");

        let mut input_horibox = input_horibox.clone();
        let input_label = Label::new(&ui, "Input:");
        let input_entry = Entry::new(&ui);
        let input_button = Button::new(&ui, "Push");
        input_horibox.append(&ui, input_entry.clone(), LayoutStrategy::Stretchy);
        input_horibox.append(&ui, input_button.clone(), LayoutStrategy::Compact);

        vertbox.append(&ui, source_label.clone(), LayoutStrategy::Compact);
        vertbox.append(&ui, source_multi.clone(), LayoutStrategy::Stretchy);
        vertbox.append(&ui, run_horibox, LayoutStrategy::Compact);
        vertbox.append(&ui, result_label, LayoutStrategy::Compact);
        vertbox.append(&ui, result_multi.clone(), LayoutStrategy::Stretchy);
        vertbox.append(&ui, input_label, LayoutStrategy::Compact);
        vertbox.append(&ui, input_horibox, LayoutStrategy::Compact);

        (source_label, source_multi, run_button, stop_button, result_multi, input_entry, input_button)
    };

    let file_menu = Menu::new(&ui, "File");
    file_menu.append_item("Open File...")
        .on_clicked(&ui, |_, w| {
            let path_result = w.open_file(&ui);
            if let Some(pathbuf) = path_result {
                let result = fs::read_to_string(&pathbuf);
                if let Ok(source) = result {
                    let mut source_label = source_label.clone();
                    source_label.set_text(&ui, &("Source: ".to_string() + pathbuf.file_name().unwrap().to_str().unwrap()));
                    let mut source_multi = source_multi.clone();
                    source_multi.set_value(&ui, &source);
                } else {
                    w.modal_err(&ui, "Opening file is failed", "Opening file is failed");
                }
            }
        });
    file_menu.append_item("Save As...")
        .on_clicked(&ui, |_, w| {
            let path_result = w.save_file(&ui);
            if let Some(pathbuf) = path_result {
                let mut source_label = source_label.clone();
                let source_multi = source_multi.clone();
                let source = source_multi.value(&ui);
                if !source.is_empty() {
                    match fs::File::create(&pathbuf) {
                        Ok(mut file) => {
                            if let Err(_) = write!(file, "{}", &source) {
                                w.modal_err(&ui, "Writing into file failed", "Writing into file failed");
                            } else {
                                source_label.set_text(&ui, &("Source: ".to_string() + pathbuf.file_name().unwrap().to_str().unwrap()));
                            }
                        },
                        Err(_) => { w.modal_err(&ui, "File creation failed", "File creation failed"); },
                    }
                }
            } else {
                w.modal_err(&ui, "Invalid path used", "Invalid path used");
            }
        });

    run_button.on_clicked(&ui, |_| {
        let app_state = app_state.clone();
        app_state.borrow_mut().stop = false;
        app_state.borrow_mut().running = true;
    });
    stop_button.on_clicked(&ui, |_| {
        let app_state = app_state.clone();
        app_state.borrow_mut().stop = true;
        app_state.borrow_mut().running = true;
    });
    input_button.on_clicked(&ui, |_| {
        let app_state = app_state.clone();
        app_state.borrow_mut().running = true;
        app_state.borrow_mut().input_exists = true;
    });

    let mut win = Window::new(&ui, "BrainFucker", 800, 600, WindowType::HasMenubar);
    win.set_child(&ui, vertbox);
    win.show(&ui);

    let mut event_loop = ui.event_loop();
    let pool = ThreadPool::new().expect("thread-pool creation failed.");
    let (mut abort_handle, _) = AbortHandle::new_pair();
    let (tx, mut rx) = channel::<Result<Brainfuck, &'static str>>(0);

    loop {
        let source_multi = source_multi.clone();
        let mut result_multi = result_multi.clone();
        let mut input_entry = input_entry.clone();
        let app_state = app_state.clone();

        if app_state.borrow().stop {
            if app_state.borrow().running {
                let console = result_multi.value(&ui);
                result_multi.set_value(&ui, &(console + "\n[Interrupt]"));
                app_state.borrow_mut().stop = false;
                app_state.borrow_mut().bf_futures.clear();
                abort_handle.abort();
                while let Ok(_) = rx.try_next() {}
                app_state.borrow_mut().running = false;
                app_state.borrow_mut().input_exists = false;
            }
        }

        // Receiverが結果を持っていれば、それをGUIに反映する
        if app_state.borrow().running {
            if let Ok(Some(bf_result)) = rx.try_next() {
                match bf_result {
                    Ok(mut bf) => {
                        if bf.reach_eop() {
                            let console = result_multi.value(&ui);
                            result_multi.set_value(&ui, &(console + "\n" + &(bf.pop_result())));
                            app_state.borrow_mut().running = false;
                            app_state.borrow_mut().bf_futures.clear();
                        } else {
                            if bf.is_input_mode() {
                                if app_state.borrow().input_exists {
                                    let input = input_entry.value(&ui);
                                    if !input.is_empty() {
                                        match bf.set_input(input.clone()) {
                                            Ok(_) => {
                                                let console = result_multi.value(&ui);
                                                result_multi.set_value(&ui, &(console + &input));
                                                input_entry.set_value(&ui, "");
                                                app_state.borrow_mut().bf_futures.clear();
                                                while let Ok(_) = rx.try_next() {}
                                                let tx = tx.clone();
                                                let (bf_future, ah) = abortable(bf_interpret(bf, tx));
                                                abort_handle = ah;
                                                app_state.borrow_mut().bf_futures.push(
                                                    pool.spawn_with_handle(bf_future).unwrap()
                                                );
                                            },
                                            Err(err) => {
                                                let console = result_multi.value(&ui);
                                                result_multi.set_value(&ui, &(console + "\n[Interpreter Error: " + err + "]"));
                                                app_state.borrow_mut().bf_futures.clear();
                                                app_state.borrow_mut().running = false;
                                            }
                                        }
                                        app_state.borrow_mut().input_exists = false;
                                    }
                                } else {
                                    let result = bf.pop_result();
                                    if !result.is_empty() {
                                        let console = result_multi.value(&ui);
                                        result_multi.set_value(&ui, &(console + "\n" + &result));
                                    }
                                    let console = result_multi.value(&ui);
                                    result_multi.set_value(&ui, &(console + "\n[Input] <- "));
                                    app_state.borrow_mut().bf_futures.clear();
                                    while let Ok(_) = rx.try_next() {}
                                    let tx = tx.clone();
                                    let (bf_future, ah) = abortable(bf_interpret(bf, tx));
                                    abort_handle = ah;
                                    app_state.borrow_mut().bf_futures.push(
                                        pool.spawn_with_handle(bf_future).unwrap()
                                    );
                                    app_state.borrow_mut().running = false;
                                }
                            }
                        }
                    },
                    Err(err) => {
                        let console = result_multi.value(&ui);
                        result_multi.set_value(&ui, &(console + "\n[Interpreter Error: " + err + "]"));
                        app_state.borrow_mut().running = false;
                        app_state.borrow_mut().bf_futures.clear();
                        while let Ok(_) = rx.try_next() {}
                    }
                }
            }
        }

        if event_loop.next_event_tick(&ui) {
            if app_state.borrow().running {
                if app_state.borrow().bf_futures.len() == 0 {
                    let program = source_multi.value(&ui);
                    if !program.is_empty() {
                        match Brainfuck::new(program) {
                            Ok(bf) => {
                                // ここでは、初期化された状態のBrainfuckインタプリタから実行を始めさせる
                                let tx = tx.clone();
                                let (bf_future, ah) = abortable(bf_interpret(bf, tx));
                                abort_handle = ah;
                                app_state.borrow_mut().bf_futures.push(
                                    pool.spawn_with_handle(bf_future).unwrap()
                                );
                            },
                            Err(err) => {
                                let console = result_multi.value(&ui);
                                result_multi.set_value(&ui, &(console + "\n[Interpreter Error: " + err + "]"));
                            }
                        }
                    }
                }
            }
        } else {
            break;
        }
    }
}

async fn bf_interpret(mut bf: Brainfuck, mut tx: Sender<Result<Brainfuck, &'static str>>) {
    if !(bf.include_comma()) {
        match bf.step_loop() {
            Ok(_) => tx.try_send(Ok(bf)).unwrap(),
            Err(err) => tx.try_send(Err(err)).unwrap(),
        }
    } else {
        while !(bf.is_input_mode() && bf.queue_remain() == 0) {
            if bf.is_input_mode() {
                // queue_remain != 0 でも、空の文字列を補充しなければならない（input_mode を false にするため）
                if let Err(err) = bf.set_input(String::new()) {
                    tx.try_send(Err(err)).unwrap();
                    break;
                }
            } else {
                if bf.reach_eop() {
                    break;
                } else {
                    if let Err(err) = bf.step() {
                        tx.try_send(Err(err)).unwrap();
                        break;
                    }
                }
            }
        }
        tx.try_send(Ok(bf)).unwrap();
    }
}
