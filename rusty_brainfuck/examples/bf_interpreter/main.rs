extern crate rusty_brainfuck;

mod interpret_io {
    use std::io;
    use std::io::Write;
    use std::fs::File;
    use std::io::Read;

    type Filename = String;
    type Program = String;

    use rusty_brainfuck::Brainfuck;

    enum IsContinue {
        Yes,
        No,
    }

    use IsContinue::{Yes, No};

    pub fn print_discription() {
        println!("Brainfuck Interpreter - interactive mode");
        println!("    Usage: Input a brainfuck code, then press 'Enter'.");
        println!("           For terminating this interpreter, input 'q'");
        println!("           then press 'Enter'.");
    }

    pub fn print_discription_at_file() {
        println!("Brainfuck Interpreter - file execution mode");
    }

    pub fn exec_loop() {
        loop {
            match wait_input() {
                Yes => (),
                No  => { break; },
            }
        }
    }

    pub fn file_exec(filename: Filename) {
        let mut program = Program::new();
        match File::open(filename).as_mut() {
            Ok(file) => {
                match file.read_to_string(&mut program) {
                    Ok(_) => {
                        if program.is_empty() {
                            println!("Thie file is empty.");
                        } else {
                            if let Err(msg) = exec_and_input(program) {
                                println!("error: {}", msg);
                            }
                        }
                    },
                    Err(err) => { println!("error: {}", err); },
                }
            }
            Err(err) => { println!("error: {}", err); },
        }
    }

    fn wait_input() -> IsContinue {
        print!("$ input : ");
        io::stdout().flush().unwrap();
        let mut program = Program::new();
        match io::stdin().read_line(&mut program) {
            Ok(_) => {
                // q が入力されたら終了
                if program.trim() == "q" {
                    println!("See you!");
                    return No;
                }
                match exec_and_input(program) {
                    Ok(_) => Yes,
                    Err(err) => {
                        println!("error: {}", err);
                        No
                    },
                }
            },
            Err(error) => {
                println!("error: {}", error);
                No
            },
        }
    }

    fn exec_and_input(program: Program) -> Result<(), &'static str>{
        let mut bf = Brainfuck::new(program)?;
        let bf_include_comma = bf.include_comma();
        loop {
            // ステップ実行後、"," を踏んでいれば入力モードになっている
            if !bf.is_input_mode() {
                // Brainfuck の prorgram counter が end of program に到達していたら、
                // 結果を表示してループを抜ける
                if bf.reach_eop() {
                    println!("{}", bf.pop_result());
                    break;
                }
                // 入力モードかどうかのチェックを行うだけでパフォーマンスが落ちるので、
                // `,` が含まれていなければ入力モードのチェックなしにノンストップで step loop を回す
                if !bf_include_comma {
                    if let Err(msg) = bf.step_loop() {
                        println!("error: {}", msg);
                        break;
                    }
                } else {
                    if let Err(msg) = bf.step() {
                        println!("error: {}", msg);
                        break;
                    }
                }
            } else {
                let mut input = String::new();
                // input queue が空のとき、input に入力を補充する
                if bf.queue_remain() == 0 {
                    let result = bf.pop_result();
                    if !result.is_empty() {
                        println!("{}", result);
                    }
                    print!("$ input queue <- ");
                    io::stdout().flush().unwrap();
                    match io::stdin().read_line(&mut input) {
                        Ok(_)    => (),
                        Err(msg) => { println!("error: {}", msg); break; },
                    }
                    input = input.trim().to_string();
                }
                if let Err(msg) = bf.set_input(input) {
                    println!("error: {}", msg);
                    break;
                }
            }
        }
        Ok(())
    }
}

use std::env;
use interpret_io::{
    print_discription,
    print_discription_at_file,
    exec_loop,
    file_exec,
};

fn main() {
    if env::args().count() < 2 {
        print_discription();
        exec_loop();
    } else {
        print_discription_at_file();
        let filename = env::args().last().unwrap();
        file_exec(filename);
    }
}
