use std::iter::Peekable;
use std::str::Chars;
use std::ops::DerefMut;

use Inst::*;

type ProgramString = String;
type ResultString  = String;
type InputString   = String;

const MAX_MEMORY: usize = 30_000;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Inst {
    Add(u8),       // Add a number to the current address
    Sub(u8),       // Sub a number to the current address
    Shr(usize),    // Shift the pointer to the right
    Shl(usize),    // Shift the pointer to the left
    Jpf(usize),    // Jump forward (to the closing bracket)
    Jpb(usize),    // Jump back (to the open bracket)
    Wrt,           // Display the current pointee
    Red,           // Get a input, and put the input to the current address
    Stz,           // Store 0 to the current address
}

struct CodeGen<'a> {
    chars: Box<Peekable<Chars<'a>>>,
}

impl<'a> CodeGen<'a> {
    pub fn new(program: &'a str) -> CodeGen<'a> {
        CodeGen { chars: Box::new(program.chars().peekable()) }
    }

    pub fn generate_insts(&mut self) -> Result<Vec<Inst>, &'static str> {
        let chars = self.chars.deref_mut();
        let mut result = Vec::new();

        let mut pos: usize = 0;

        'outer: loop {
            {
                let ch = chars.peek();
                if ch.is_none() {
                    break 'outer;
                }
            }

            match chars.next().unwrap() {
                '+' => {
                    let start = pos;
                    'add: loop {
                        let ch = match chars.peek() {
                            Some(ch) => *ch,
                            None => {
                                result.push(Add((pos - start + 1) as u8));
                                break 'outer
                            }
                        };

                        if ch != '+' {
                            result.push(Add((pos - start + 1) as u8));
                            break 'add;
                        }

                        chars.next();
                        pos += 1;
                    }
                },
                '-' => {
                    let start = pos;
                    'sub: loop {
                        let ch = match chars.peek() {
                            Some(ch) => *ch,
                            None => {
                                result.push(Sub((pos - start + 1) as u8));
                                break 'outer
                            }
                        };

                        if ch != '-' {
                            result.push(Sub((pos - start + 1) as u8));
                            break 'sub;
                        }

                        chars.next();
                        pos += 1;
                    }
                },
                '>' => {
                    let start = pos;
                    'shr: loop {
                        let ch = match chars.peek() {
                            Some(ch) => *ch,
                            None => {
                                result.push(Shr(pos - start + 1));
                                break 'outer
                            }
                        };

                        if ch != '>' {
                            result.push(Shr(pos - start + 1));
                            break 'shr;
                        }

                        chars.next();
                        pos += 1;
                    }
                },
                '<' => {
                    let start = pos;
                    'shl: loop {
                        let ch = match chars.peek() {
                            Some(ch) => *ch,
                            None => {
                                result.push(Shl(pos - start + 1));
                                break 'outer
                            }
                        };

                        if ch != '<' {
                            result.push(Shl(pos - start + 1));
                            break 'shl;
                        }

                        chars.next();
                        pos += 1;
                    }
                },
                '[' => result.push(Jpf(0)),
                ']' => result.push(Jpb(0)),
                '.' => result.push(Wrt),
                ',' => result.push(Red),
                _ => (),
            }
        }

        if result.len() > 3 {
            let mut temp_v = Vec::new();
            let mut iter3w = result.windows(3).peekable();
            let mut pushed_num = 0;

            while let Some(three) = iter3w.next() {
                if three == &[Jpf(0), Sub(1), Jpb(0)] {
                    temp_v.push(Stz);
                    pushed_num = 2;
                } else {
                    if pushed_num > 0 {
                        pushed_num -= 1;
                    } else {
                        temp_v.push(three[0]);
                    }
                }
                if let Some(_) = iter3w.peek() {
                    continue;
                } else {
                    if pushed_num == 1 {
                        temp_v.push(three[2]);
                    } else if pushed_num != 2 {
                        temp_v.push(three[1]);
                        temp_v.push(three[2]);
                    }
                }
            }

            result = temp_v;
        }

        for counter in 0..result.len() {
            let inst = result[counter];
            if let Jpf(_) = inst {
                let mut staple = 1;
                let mut seek = counter;
                while staple != 0 && seek < result.len() {
                    seek += 1;
                    let inst2 = result[seek];
                    match inst2 {
                        Jpb(_) => { staple -= 1; },
                        Jpf(_) => { staple += 1; },
                         _  => (),
                    }
                }
                if staple == 0 {
                    result[counter] = Jpf(seek);
                    result[seek] = Jpb(counter);
                } else {
                    // println!("{:?}", jumptable);
                    return Err("Jumptable can't be constructed.");
                }
            }
        }

        Ok(result)
    }
}

#[derive(Copy, Clone)]
struct Pointer(usize);

impl Pointer {
    fn shift_right_n(&mut self, n: usize) -> Result<(), &'static str> {
        if self.0 + n < MAX_MEMORY {
            self.0 += n;
            Ok(())
        } else {
            Err("Too large pointer than the size of memory.")
        }
    }

    fn shift_left_n(&mut self, n: usize) -> Result<(), &'static str> {
        if self.0 >= n {
            self.0 -= n;
            Ok(())
        } else {
            Err("Too small pointer than the first address of memory.")
        }
    }
}

impl From<Pointer> for usize {
    fn from(pointer: Pointer) -> Self {
        pointer.0
    }
}

impl From<usize> for Pointer {
    fn from(num: usize) -> Self {
        if num >= MAX_MEMORY {
            panic!("usize-num is out of range of Pointer.");
        }
        Pointer(num)
    }
}

#[derive(Copy, Clone)]
struct Counter {
    index: usize,
    max_index: usize,
}

impl Counter {
    fn new(index: usize, max_index: usize) -> Self {
        Counter {
            index,
            max_index,
        }
    }

    fn inc(&mut self) -> Result<(), &'static str> {
        if self.index <= self.max_index {
            self.index += 1;
            Ok(())
        } else {
            Err("Counter is larger than the program size.")
        }
    }

    fn jump(&mut self, index: usize) -> Result<(), &'static str> {
        if index < self.max_index {
            self.index = index;
            Ok(())
        } else {
            Err("Counter is larger than the program size.")
        }
    }

    fn index(&self) -> usize {
        self.index
    }

    fn is_max(&self) -> bool {
        self.index == self.max_index
    }
}

pub struct Brainfuck {
    insts:       Vec<Inst>,
    result:      ResultString,
    memory:      Vec<u8>,
    pointer:     Pointer,
    counter:     Counter,
    input_queue: InputString,
    input_mode:  bool,
}

impl Brainfuck {
    pub fn new(program: ProgramString) -> Result<Self, &'static str> {
        let mut program = program;
        Brainfuck::serialize(&mut program);
        let mut codegen = CodeGen::new(&program);
        let insts = codegen.generate_insts()?;
        let insts_len = insts.len();
        Ok(Brainfuck {
            insts,
            result:      ResultString::new(),
            memory:      vec![0; MAX_MEMORY],
            pointer:     0.into(),
            counter:     Counter::new(0, insts_len),
            input_queue: InputString::new(),
            input_mode:  false,
        })
    }

    fn serialize(program: &mut ProgramString) {
        program.retain(|c| c == '+' || c == '-' || c == '>' || c == '<' ||
                       c == '[' || c == ']' || c == '.' || c == ',');
    }

    pub fn initialize(&mut self, program: ProgramString) -> Result<(), &'static str> {
        let mut program = program;
        Brainfuck::serialize(&mut program);
        let mut codegen = CodeGen::new(&program);
        let insts = codegen.generate_insts()?;
        let insts_len = insts.len();
        self.insts = insts;
        self.result = ResultString::new();
        self.memory = vec![0; MAX_MEMORY];
        self.pointer = 0.into();
        self.counter = Counter::new(0, insts_len);
        self.input_queue = InputString::new();
        self.input_mode = false;
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), &'static str> {
        let inst = self.insts.as_slice()[self.counter.index()];

        match inst {
            Add(n) => self.value_plus(n),
            Sub(n) => self.value_minus(n),
            Shr(n) => self.pointer_shift_right(n)?,
            Shl(n) => self.pointer_shift_left(n)?,
            Wrt => self.push_from_memory_into_result(),
            Jpf(idx) => self.jump_to_close_staple(idx)?,
            Jpb(idx) => self.jump_to_start_staple(idx)?,
            Red => { self.input_mode = true; ()},
            Stz => self.store_zero(),
        }

        self.counter.inc()?;
        Ok(())
    }

    fn store_zero(&mut self) {
        let pointer: usize = self.pointer.into();
        self.memory[pointer] = 0;
    }

    fn value_plus(&mut self, n: u8) {
        let pointer: usize = self.pointer.into();
        self.memory[pointer] = self.memory[pointer].wrapping_add(n);
    }

    fn value_minus(&mut self, n: u8) {
        let pointer: usize = self.pointer.into();
        self.memory[pointer] = self.memory[pointer].wrapping_sub(n);
    }

    fn pointer_shift_right(&mut self, n: usize) -> Result<(), &'static str> {
        self.pointer.shift_right_n(n)?;
        Ok(())
    }

    fn pointer_shift_left(&mut self, n: usize) -> Result<(), &'static str> {
        self.pointer.shift_left_n(n)?;
        Ok(())
    }

    fn push_from_memory_into_result(&mut self) {
        let pointer: usize = self.pointer.into();
        let out_char: char = self.memory[pointer].into();
        self.result.push(out_char);
    }

    fn jump_to_close_staple(&mut self, index: usize) -> Result<(), &'static str> {
        let pointer: usize = self.pointer.into();
        let value = self.memory[pointer];
        if value == 0 {
            self.counter.jump(index)?;
        }
        Ok(())
    }

    fn jump_to_start_staple(&mut self, index: usize) -> Result<(), &'static str> {
        let pointer: usize = self.pointer.into();
        let value = self.memory[pointer];
        if value != 0 {
            self.counter.jump(index)?;
        }
        Ok(())
    }

    pub fn queue_remain(&self) -> i32 {
        self.input_queue.len() as i32
    }

    pub fn is_input_mode(&self) -> bool {
        self.input_mode
    }

    pub fn set_input(&mut self, input: InputString) -> Result<(), &'static str> {
        if input.len() == 0 && self.queue_remain() <= 0 {
            Err("input empty String.")
        } else {
            self.input_queue += &input;
            let c = self.input_queue.remove(0) as i32;
            match c {
                0..=127 => {
                    let pointer: usize = self.pointer.into();
                    self.memory[pointer] = c as u8;
                },
                _ => { return Err("Input contains non-ascii code."); },
            }
            self.input_mode = false;
            Ok(())
        }
    }

    pub fn pop_result(&mut self) -> ResultString {
        let result = self.result.clone();
        self.result.clear();
        result
    }

    pub fn reach_eop(&self) -> bool {
        self.counter.is_max()
    }

    pub fn include_comma(&self) -> bool {
        self.insts.contains(&Red)
    }

    pub fn step_loop(&mut self) -> Result<(), &'static str> {
        loop {
            if self.reach_eop() {
                break;
            }
            self.step()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_overflow() {
        let program = ">".repeat(30_000);
        let mut bf = Brainfuck::new(program).unwrap();
        assert_eq!(bf.step(), Err("Too large pointer than the size of memory."));
    }

    #[test]
    fn pointer_minus() {
        let program = String::from("<");
        let mut bf = Brainfuck::new(program).unwrap();
        assert_eq!(bf.step(), Err("Too small pointer than the first address of memory."));
    }
}
