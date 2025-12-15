//! # Virtual Machine
//!
//! This module implements a simple stack-based virtual machine for executing
//! the C subset. It includes a bytecode compiler that translates the AST
//! into a linear sequence of instructions.
//!
//! ## Architecture
//!
//! - **Value**: Supports `Int` (i64) and `Float` (f64).
//! - **Stack**: Used for operand storage and expression evaluation.
//! - **Call Stack**: Manages function calls, local variables, and return addresses.
//! - **Instruction Set**: Arithmetic, comparison, control flow, and function calls.

use std::collections::HashMap;
use crate::ast::{BinOp, Expr, Function, Literal, Program, Stmt};

/// Represents a runtime value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Void,
}

impl Value {
    fn is_truthy(&self) -> bool {
        match self {
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Void => false,
        }
    }
}

/// Bytecode instructions.
#[derive(Debug, Clone, PartialEq)]
pub enum Opcode {
    /// Push a constant value onto the stack.
    LoadConst(Value),
    /// Load a variable's value onto the stack.
    LoadVar(String),
    /// Store the top of the stack into a variable.
    StoreVar(String),
    /// Binary operation (add, sub, mul, div, etc.).
    BinaryOp(BinOp),
    /// Unconditional jump to an instruction index.
    Jump(usize),
    /// Jump if the top of the stack is false (0).
    JumpIfFalse(usize),
    /// Call a function by name with n arguments.
    Call(String, usize),
    /// Return from the current function.
    Return,
    /// Pop a value from the stack (used for expression statements).
    Pop,
    /// Halt execution.
    Halt,
}

/// The Virtual Machine.
pub struct VM {
    /// Instruction memory.
    code: Vec<Opcode>,
    /// Operand stack.
    stack: Vec<Value>,
    /// Call stack (frames).
    frames: Vec<Frame>,
    /// Instruction pointer.
    ip: usize,
    /// Map of function names to their entry point addresses.
    functions: HashMap<String, usize>,
}

/// A stack frame for a function call.
struct Frame {
    /// Return address (instruction pointer).
    return_ip: usize,
    /// Local variables.
    locals: HashMap<String, Value>,
}

impl VM {
    /// Creates a new VM with the given program.
    pub fn new(program: &Program) -> Self {
        let (code, functions) = Compiler::compile(program);
        Self {
            code,
            stack: Vec::new(),
            frames: Vec::new(),
            ip: 0,
            functions,
        }
    }

    /// Executes the program.
    /// Returns the exit code (return value of main).
    pub fn run(&mut self) -> Result<Value, String> {
        // Start at main
        if let Some(&start_addr) = self.functions.get("main") {
            self.ip = start_addr;
            // Push a dummy frame for main so it has somewhere to return (or halt)
            self.frames.push(Frame {
                return_ip: self.code.len(), // Point to Halt or end of code
                locals: HashMap::new(),
            });
        } else {
            return Err("Function 'main' not found".to_string());
        }

        loop {
            if self.ip >= self.code.len() {
                break;
            }

            let op = self.code[self.ip].clone();
            self.ip += 1;

            match op {
                Opcode::LoadConst(val) => self.stack.push(val),
                Opcode::LoadVar(name) => {
                    if let Some(frame) = self.frames.last() {
                        if let Some(val) = frame.locals.get(&name) {
                            self.stack.push(*val);
                        } else {
                            return Err(format!("Undefined variable: {}", name));
                        }
                    } else {
                        return Err("No active stack frame".to_string());
                    }
                }
                Opcode::StoreVar(name) => {
                    if let Some(val) = self.stack.pop() {
                        if let Some(frame) = self.frames.last_mut() {
                            frame.locals.insert(name, val);
                        } else {
                            return Err("No active stack frame".to_string());
                        }
                    } else {
                        return Err("Stack underflow".to_string());
                    }
                }
                Opcode::BinaryOp(op) => {
                    let right = self.stack.pop().ok_or("Stack underflow")?;
                    let left = self.stack.pop().ok_or("Stack underflow")?;
                    let result = self.eval_binary(left, op, right);
                    self.stack.push(result);
                }
                Opcode::Jump(target) => self.ip = target,
                Opcode::JumpIfFalse(target) => {
                    let val = self.stack.pop().ok_or("Stack underflow")?;
                    if !val.is_truthy() {
                        self.ip = target;
                    }
                }
                Opcode::Call(name, _arg_count) => {
                    if let Some(&addr) = self.functions.get(&name) {
                        // Arguments are already on the stack.
                        // The function prologue (generated by Compiler) will pop them into local variables.
                        
                        self.frames.push(Frame {
                            return_ip: self.ip,
                            locals: HashMap::new(),
                        });
                        self.ip = addr;
                    } else {
                        return Err(format!("Undefined function: {}", name));
                    }
                }
                Opcode::Return => {
                    if let Some(frame) = self.frames.pop() {
                        self.ip = frame.return_ip;
                        // If we returned from main (stack empty or we hit the end), we are done.
                        if self.frames.is_empty() {
                            // Return the top of stack as result
                            return Ok(self.stack.pop().unwrap_or(Value::Void));
                        }
                    } else {
                        return Err("Call stack underflow".to_string());
                    }
                }
                Opcode::Pop => {
                    self.stack.pop();
                }
                Opcode::Halt => break,
            }
        }
        
        Ok(self.stack.pop().unwrap_or(Value::Void))
    }

    fn eval_binary(&self, left: Value, op: BinOp, right: Value) -> Value {
        match (left, right) {
            (Value::Int(l), Value::Int(r)) => match op {
                BinOp::Plus => Value::Int(l + r),
                BinOp::Minus => Value::Int(l - r),
                BinOp::Multiply => Value::Int(l * r),
                BinOp::Divide => Value::Int(l / r),
                BinOp::Equal => Value::Int((l == r) as i64),
                BinOp::NotEqual => Value::Int((l != r) as i64),
                BinOp::LessThan => Value::Int((l < r) as i64),
                BinOp::GreaterThan => Value::Int((l > r) as i64),
                BinOp::LessEqual => Value::Int((l <= r) as i64),
                BinOp::GreaterEqual => Value::Int((l >= r) as i64),
            },
            (Value::Float(l), Value::Float(r)) => match op {
                BinOp::Plus => Value::Float(l + r),
                BinOp::Minus => Value::Float(l - r),
                BinOp::Multiply => Value::Float(l * r),
                BinOp::Divide => Value::Float(l / r),
                BinOp::Equal => Value::Int((l == r) as i64),
                BinOp::NotEqual => Value::Int((l != r) as i64),
                BinOp::LessThan => Value::Int((l < r) as i64),
                BinOp::GreaterThan => Value::Int((l > r) as i64),
                BinOp::LessEqual => Value::Int((l <= r) as i64),
                BinOp::GreaterEqual => Value::Int((l >= r) as i64),
            },
            // Mixed types (promote Int to Float)
            (Value::Int(l), Value::Float(r)) => self.eval_binary(Value::Float(l as f64), op, Value::Float(r)),
            (Value::Float(l), Value::Int(r)) => self.eval_binary(Value::Float(l), op, Value::Float(r as f64)),
            _ => Value::Void, // Should be caught by semantic analysis
        }
    }
}

/// Compiler from AST to Bytecode.
struct Compiler {
    code: Vec<Opcode>,
    functions: HashMap<String, usize>,
}

impl Compiler {
    fn compile(program: &Program) -> (Vec<Opcode>, HashMap<String, usize>) {
        let mut compiler = Compiler {
            code: Vec::new(),
            functions: HashMap::new(),
        };
        
        // First pass: record function entry points (placeholders)
        // Actually, we can just generate code sequentially.
        // But we need to handle forward calls?
        // In this simple VM, we can just generate all functions.
        // Calls will look up the address in the map.
        
        // We need a "start" or "main" entry point.
        // The VM run loop looks for "main".
        
        for function in &program.functions {
            compiler.functions.insert(function.name.clone(), compiler.code.len());
            compiler.compile_function(function);
        }
        
        (compiler.code, compiler.functions)
    }

    fn compile_function(&mut self, function: &Function) {
        // Handle parameters: they are on the stack in reverse order.
        // We need to store them in local variables.
        // Example: add(a, b). Stack: [a, b].
        // We need to pop b -> store b, pop a -> store a.
        // So we iterate params in reverse.
        for (_, name) in function.params.iter().rev() {
            self.code.push(Opcode::StoreVar(name.clone()));
        }

        self.compile_stmt(&function.body);
        
        // Ensure explicit return at end of function if missing
        if !matches!(self.code.last(), Some(Opcode::Return)) {
             self.code.push(Opcode::LoadConst(Value::Int(0))); // Default return 0
             self.code.push(Opcode::Return);
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Declaration { ty: _, name, init } => {
                if let Some(expr) = init {
                    self.compile_expr(expr);
                    self.code.push(Opcode::StoreVar(name.clone()));
                }
            }
            Stmt::Assignment { name, expr } => {
                self.compile_expr(expr);
                self.code.push(Opcode::StoreVar(name.clone()));
            }
            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.compile_expr(e);
                } else {
                    self.code.push(Opcode::LoadConst(Value::Void));
                }
                self.code.push(Opcode::Return);
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.compile_stmt(s);
                }
            }
            Stmt::If { cond, then, else_ } => {
                self.compile_expr(cond);
                
                let jump_if_false_idx = self.code.len();
                self.code.push(Opcode::JumpIfFalse(0)); // Placeholder
                
                self.compile_stmt(then);
                
                let jump_end_idx = self.code.len();
                self.code.push(Opcode::Jump(0)); // Placeholder
                
                // Patch JumpIfFalse
                let else_start = self.code.len();
                self.code[jump_if_false_idx] = Opcode::JumpIfFalse(else_start);
                
                if let Some(else_stmt) = else_ {
                    self.compile_stmt(else_stmt);
                }
                
                // Patch Jump (end of then block)
                let end = self.code.len();
                self.code[jump_end_idx] = Opcode::Jump(end);
            }
            Stmt::For { init, cond, update, body } => {
                if let Some(init_stmt) = init {
                    self.compile_stmt(init_stmt);
                }
                
                let loop_start = self.code.len();
                
                // Condition
                let jump_end_idx = if let Some(cond_expr) = cond {
                    self.compile_expr(cond_expr);
                    let idx = self.code.len();
                    self.code.push(Opcode::JumpIfFalse(0)); // Placeholder
                    Some(idx)
                } else {
                    None
                };
                
                self.compile_stmt(body);
                
                if let Some(update_expr) = update {
                    self.compile_expr(update_expr);
                    self.code.push(Opcode::Pop); // Discard update result
                }
                
                self.code.push(Opcode::Jump(loop_start));
                
                // Patch exit jump
                if let Some(idx) = jump_end_idx {
                    let end = self.code.len();
                    self.code[idx] = Opcode::JumpIfFalse(end);
                }
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr);
                self.code.push(Opcode::Pop);
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Int(i) => self.code.push(Opcode::LoadConst(Value::Int(*i))),
                Literal::Float(f) => self.code.push(Opcode::LoadConst(Value::Float(*f))),
            },
            Expr::Identifier(name) => self.code.push(Opcode::LoadVar(name.clone())),
            Expr::Binary { left, op, right } => {
                self.compile_expr(left);
                self.compile_expr(right);
                self.code.push(Opcode::BinaryOp(*op));
            }
            Expr::Call { name, args } => {
                for arg in args {
                    self.compile_expr(arg);
                }
                self.code.push(Opcode::Call(name.clone(), args.len()));
            }
        }
    }
}

