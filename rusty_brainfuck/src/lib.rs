type ProgramString = String;
type ResultString  = String;
type InputString   = String;

const MAX_MEMORY: usize = 30_000;

#[derive(Copy, Clone)]
struct Pointer(usize);

impl Pointer {
    fn shift_right(&mut self) -> Result<(), &'static str> {
        if self.0 < MAX_MEMORY - 1 {
            self.0 += 1;
            Ok(())
        } else {
            Err("Too large pointer than the size of memory.")
        }
    }

    fn shift_left(&mut self) -> Result<(), &'static str> {
        if self.0 > 0 {
            self.0 -= 1;
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

    fn dec(&mut self) -> Result<(), &'static str> {
        if self.index > 0 {
            self.index -= 1;
            Ok(())
        } else {
            Err("Counter is smaller than zero.")
        }
    }

    fn index(&self) -> usize {
        self.index
    }

    fn is_max(&self) -> bool {
        self.index == self.max_index
    }
}

// use std::time::Instant;

pub struct Brainfuck {
    program:     ProgramString,
    result:      ResultString,
    memory:      Vec<u8>,
    pointer:     Pointer,
    counter:     Counter,
    jumptable:   Vec<usize>,
    input_queue: InputString,
    input_mode:  bool,
}

impl Brainfuck {
    pub fn new(program: ProgramString) -> Result<Self, &'static str> {
        let program_len = program.len();
        let jumptable = Brainfuck::init_jumptable(&program)?;
        Ok(Brainfuck {
            program,
            result:      ResultString::new(),
            memory:      vec![0; MAX_MEMORY],
            pointer:     0.into(),
            counter:     Counter::new(0, program_len),
            jumptable,
            input_queue: InputString::new(),
            input_mode:  false,
        })
    }

    pub fn initialize(&mut self, program: ProgramString) -> Result<(), &'static str> {
        let program_len = program.clone().len();
        self.program = program.clone();
        self.result = ResultString::new();
        self.memory = vec![0; MAX_MEMORY];
        self.pointer = 0.into();
        self.counter = Counter::new(0, program_len);
        self.jumptable = Brainfuck::init_jumptable(&program)?;
        self.input_queue = InputString::new();
        self.input_mode = false;
        Ok(())
    }

    fn init_jumptable(program: &ProgramString) -> Result<Vec<usize>, &'static str> {
        let mut jumptable = vec![0; program.len()];
        let program = program.as_bytes();
        for counter in 0..program.len() {
            let bf_symbol: char = program[counter].into();
            if bf_symbol == '[' {
                let mut staple = 1;
                let mut seek = counter;
                while staple != 0 && seek < program.len() {
                    seek += 1;
                    let bf_symbol2: char = program[seek].into();
                    match bf_symbol2 {
                        ']' => { staple -= 1; },
                        '[' => { staple += 1; },
                         _  => (),
                    }
                }
                if staple == 0 {
                    jumptable[counter] = seek;
                    jumptable[seek] = counter;
                } else {
                    // println!("{:?}", jumptable);
                    return Err("Jumptable can't be constructed.");
                }
            }
        }
        // println!("{:?}", jumptable);
        Ok(jumptable)
    }

    pub fn step(&mut self) -> Result<(), &'static str> {
        // let start = Instant::now();
        let bf_symbol: char = self.program.as_bytes()[self.counter.index()].into();
        match bf_symbol {
            '+' => self.value_inc()?,
            '-' => self.value_dec()?,
            '>' => self.pointer_shift_right()?,
            '<' => self.pointer_shift_left()?,
            '.' => self.push_from_memory_into_result(),
            '[' => self.at_start_staple()?,
            ']' => self.jump_to_start_staple()?,
            ',' => { self.input_mode = true; ()},
             _  => (),
        };
        self.counter.inc()?;
        // let end = Instant::now();
        // println!("{}: {:?}", bf_symbol, end.duration_since(start));
        Ok(())
    }

    fn at_start_staple(&mut self) -> Result<(), &'static str> {
        let bf_symbol1 :char = self.program.as_bytes()[self.counter.index()+1].into();
        let bf_symbol2 :char = self.program.as_bytes()[self.counter.index()+2].into();
        if (bf_symbol1, bf_symbol2) == ('-', ']') {
            let pointer: usize = self.pointer.into();
            self.memory[pointer] = 0;
            self.counter.inc()?;
            self.counter.inc()?;
        } else {
            self.jump_to_close_staple()?;
        }
        Ok(())
    }

    fn value_inc(&mut self) -> Result<(), &'static str> {
        let pointer: usize = self.pointer.into();
        let mut bf_symbol: char = self.program.as_bytes()[self.counter.index()].into();
        while bf_symbol == '+' {
            self.memory[pointer] = self.memory[pointer].wrapping_add(1);
            self.counter.inc()?;
            bf_symbol = self.program.as_bytes()[self.counter.index()].into();
        }
        self.counter.dec()?;
        Ok(())
    }

    fn value_dec(&mut self) -> Result<(), &'static str> {
        let pointer: usize = self.pointer.into();
        let mut bf_symbol: char = self.program.as_bytes()[self.counter.index()].into();
        while bf_symbol == '-' {
            self.memory[pointer] = self.memory[pointer].wrapping_sub(1);
            self.counter.inc()?;
            bf_symbol = self.program.as_bytes()[self.counter.index()].into();
        }
        self.counter.dec()?;
        Ok(())
    }

    fn pointer_shift_right(&mut self) -> Result<(), &'static str> {
        let mut bf_symbol: char = self.program.as_bytes()[self.counter.index()].into();
        while bf_symbol == '>' {
            self.pointer.shift_right()?;
            self.counter.inc()?;
            bf_symbol = self.program.as_bytes()[self.counter.index()].into();
        }
        self.counter.dec()?;
        Ok(())
    }

    fn pointer_shift_left(&mut self) -> Result<(), &'static str> {
        let mut bf_symbol: char = self.program.as_bytes()[self.counter.index()].into();
        while bf_symbol == '<' {
            self.pointer.shift_left()?;
            self.counter.inc()?;
            bf_symbol = self.program.as_bytes()[self.counter.index()].into();
        }
        self.counter.dec()?;
        Ok(())
    }

    fn push_from_memory_into_result(&mut self) {
        let pointer: usize = self.pointer.into();
        let out_char: char = self.memory[pointer].into();
        self.result.push(out_char);
    }

    fn jump_to_close_staple(&mut self) -> Result<(), &'static str> {
        let pointer: usize = self.pointer.into();
        let value: i32 = self.memory[pointer].into();
        if value == 0 {
            let idx = self.counter.index();
            self.counter = Counter {
                index: self.jumptable[idx],
                ..self.counter
            }
        }
        Ok(())
    }

    fn jump_to_start_staple(&mut self) -> Result<(), &'static str> {
        let pointer: usize = self.pointer.into();
        let value: i32 = self.memory[pointer].into();
        if value != 0 {
            let idx = self.counter.index();
            self.counter = Counter {
                index: self.jumptable[idx],
                ..self.counter
            }
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
        self.program.contains(",")
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
    fn counter_overflow() {
        let program = ">".repeat(30_000);
        let mut bf = Brainfuck::new(program).unwrap();
        for _ in 0..29_999 {
            assert_eq!(bf.step(), Ok(()));
        }
        assert_eq!(bf.step(), Err("Too large pointer than the size of memory."));
    }

    #[test]
    fn counter_minus() {
        let program = String::from("<");
        let mut bf = Brainfuck::new(program).unwrap();
        assert_eq!(bf.step(), Err("Too small pointer than the first address of memory."));
    }
}
