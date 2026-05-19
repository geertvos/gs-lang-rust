use std::collections::HashSet;
use std::sync::Arc;

use gs_core::bridge::NativeMethodWrapper;
use gs_core::bridge::registry::NativeRegistry;
use gs_core::core::gvm;
use gs_core::core::types::BooleanType;
use gs_core::program::{GvmFunction, GvmProgram};
use gs_core::streams::RandomAccessByteStream;

use crate::ast::*;
use crate::lang::exception_handler::GscriptExceptionHandler;
use crate::lang::types::array_type::ArrayType;
use crate::lang::types::number::NumberType;
use crate::lang::types::object_type::ObjectType;
use crate::lang::types::string_type::StringType;
use crate::lang::value_converter::GscriptValueConverter;
use crate::runtime::NativeStaticMethodWrapper;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompilerOptimization {
    TailRecursion,
}

/// Tracks break/continue jump positions within a loop for later patching.
struct LoopInfo {
    break_positions: Vec<i32>,
    continue_positions: Vec<i32>,
}

impl LoopInfo {
    fn new() -> Self {
        LoopInfo {
            break_positions: Vec::new(),
            continue_positions: Vec::new(),
        }
    }
}

/// State for a function being compiled. Saved/restored when entering/leaving
/// nested function definitions.
struct FunctionState {
    code: RandomAccessByteStream,
    function_idx: i32,
    parameters: Vec<String>,
    locals: Vec<String>,
}

pub struct GScriptCompiler {
    pub code: RandomAccessByteStream,
    program: Option<GvmProgram>,
    natives: Vec<Box<dyn NativeMethodWrapper>>,
    registry: Arc<NativeRegistry>,
    #[allow(dead_code)]
    var_names: Vec<String>,
    loop_stack: Vec<LoopInfo>,
    enabled_optimizations: HashSet<CompilerOptimization>,
    current_module_name: String,
    current_function_idx: i32,
    current_parameters: Vec<String>,
    current_locals: Vec<String>,
    pending_debug_name: Option<String>,
}

impl GScriptCompiler {
    pub fn new(registry: Arc<NativeRegistry>) -> Self {
        let mut optimizations = HashSet::new();
        optimizations.insert(CompilerOptimization::TailRecursion);
        GScriptCompiler {
            code: RandomAccessByteStream::new(),
            program: None,
            natives: Vec::new(),
            registry,
            var_names: Vec::new(),
            loop_stack: Vec::new(),
            enabled_optimizations: optimizations,
            current_module_name: "unknown".to_string(),
            current_function_idx: -1,
            current_parameters: Vec::new(),
            current_locals: Vec::new(),
            pending_debug_name: None,
        }
    }

    fn prepare_program(&mut self) -> GvmProgram {
        let mut program = GvmProgram::new(
            "demo".to_string(),
            Box::new(GscriptExceptionHandler),
            Box::new(GscriptValueConverter),
        );
        program.register_type(Box::new(ObjectType));
        program.register_type(Box::new(StringType));
        program.register_type(Box::new(NumberType));
        program.register_type(Box::new(BooleanType));
        program.register_type(Box::new(ArrayType));
        program
    }

    pub fn compile_modules(&mut self, modules: Vec<Module>) -> GvmProgram {
        let program = self.prepare_program();
        self.program = Some(program);
        self.code = RandomAccessByteStream::new();

        let mut main_func = GvmFunction::new(self.code.clone(), Vec::new());
        main_func.set_debug_name("main".to_string());
        let idx = self.program.as_mut().unwrap().add_function(main_func);
        self.current_function_idx = idx;
        self.current_parameters = Vec::new();
        self.current_locals = Vec::new();

        // NEW Object - init main function scope
        self.code.write_byte(gvm::NEW);
        self.code.write_string("Object");

        for m in &modules {
            self.current_module_name = m.name.clone();
            let exported_names = Self::extract_exported_names(m);
            self.compile_module(m);
            self.emit_module_exports(&m.name, &exported_names);
        }

        self.code.write_byte(gvm::HALT);

        // Finalize: update the main function's bytecode
        let final_code = self.code.clone();
        self.program
            .as_mut()
            .unwrap()
            .get_function_mut(self.current_function_idx)
            .unwrap()
            .set_bytecode(final_code);
        self.program
            .as_mut()
            .unwrap()
            .set_natives(std::mem::take(&mut self.natives));

        self.program.take().unwrap()
    }

    pub fn compile(&mut self, statements: Vec<Statement>) -> GvmProgram {
        let program = self.prepare_program();
        self.program = Some(program);
        self.code = RandomAccessByteStream::new();

        let function = GvmFunction::new(self.code.clone(), Vec::new());
        let idx = self.program.as_mut().unwrap().add_function(function);
        self.current_function_idx = idx;
        self.current_parameters = Vec::new();
        self.current_locals = Vec::new();

        // NEW Object - init main function scope
        self.code.write_byte(gvm::NEW);
        self.code.write_string("Object");

        for s in &statements {
            self.compile_statement(s);
        }

        self.code.write_byte(gvm::HALT);

        let final_code = self.code.clone();
        self.program
            .as_mut()
            .unwrap()
            .get_function_mut(self.current_function_idx)
            .unwrap()
            .set_bytecode(final_code);
        self.program
            .as_mut()
            .unwrap()
            .set_natives(std::mem::take(&mut self.natives));

        self.program.take().unwrap()
    }

    #[allow(dead_code)]
    fn register_variable(&mut self, name: &str) -> i32 {
        if let Some(pos) = self.var_names.iter().position(|n| n == name) {
            return pos as i32;
        }
        self.var_names.push(name.to_string());
        (self.var_names.len() - 1) as i32
    }

    fn get_native_method_index(&mut self, arg_count: i32) -> i32 {
        // Check if we already have a wrapper with this arg count
        for (i, n) in self.natives.iter().enumerate() {
            if n.argument_count() == arg_count {
                return i as i32;
            }
        }
        let wrapper = NativeStaticMethodWrapper::new(arg_count, self.registry.clone());
        self.natives.push(Box::new(wrapper));
        (self.natives.len() - 1) as i32
    }

    fn is_enabled(&self, opt: &CompilerOptimization) -> bool {
        self.enabled_optimizations.contains(opt)
    }

    // --- Module compilation ---

    fn compile_module(&mut self, module: &Module) {
        // Create a new Object for the module scope
        // ConstantExpression() with no value = NEW Object
        self.code.write_byte(gvm::NEW);
        self.code.write_string("Object");

        // Create constructor function containing module statements + return this
        let mut func_statements = module.statements.clone();
        func_statements.push(Statement::Return {
            value: Some(Expression::This { field: None }),
            pos: module.pos,
        });

        let func_def = Expression::FunctionDef {
            parameters: Vec::new(),
            statements: func_statements,
        };
        self.pending_debug_name = Some(format!("{}.<init>", module.name));
        self.compile_expression(&func_def);

        // Invoke the constructor
        self.code.write_byte(gvm::INVOKE);
        self.code.write_int(0);

        // Assign to module name variable
        let var = Expression::Variable {
            name: module.name.clone(),
            field: None,
        };
        self.compile_expression(&var);

        self.code.write_byte(gvm::PUT);
    }

    fn extract_exported_names(module: &Module) -> Vec<String> {
        let mut names = Vec::new();
        for stmt in &module.statements {
            if let Statement::Expression(
                Expression::Assignment {
                    variable,
                    operator: None | Some(_),
                    ..
                },
                _,
            ) = stmt
            {
                if let Expression::Variable { name, field: None } = variable.as_ref() {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
            }
        }
        names
    }

    fn emit_module_exports(&mut self, module_name: &str, names: &[String]) {
        for name in names {
            // Push value: module.field
            let mod_str = self.program.as_mut().unwrap().add_string(module_name.to_string());
            self.code.write_byte(gvm::LDC_D);
            self.code.write_int(mod_str);
            self.code.write_string("String");
            self.code.write_byte(gvm::GETDYNAMIC);

            let field_str = self.program.as_mut().unwrap().add_string(name.clone());
            self.code.write_byte(gvm::LDC_D);
            self.code.write_int(field_str);
            self.code.write_string("String");
            self.code.write_byte(gvm::GET);

            // Push variable target
            let var_str = self.program.as_mut().unwrap().add_string(name.clone());
            self.code.write_byte(gvm::LDC_D);
            self.code.write_int(var_str);
            self.code.write_string("String");
            self.code.write_byte(gvm::GETDYNAMIC);

            self.code.write_byte(gvm::PUT);
            self.code.write_byte(gvm::POP);
        }
    }

    // --- Statement compilation ---

    fn compile_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Expression(expr, pos) => {
                self.emit_debug(pos);
                self.compile_expression(expr);
                self.code.write_byte(gvm::POP);
            }
            Statement::If {
                condition,
                then_clause,
                else_clause,
                pos,
            } => {
                self.emit_debug(pos);
                self.compile_expression(condition);
                self.code.write_byte(gvm::NOT);
                self.code.write_byte(gvm::CJMP);
                let else_pos = self.code.get_pointer_position();
                self.code.write_int(-1);
                self.compile_statement(then_clause);
                if let Some(else_stmt) = else_clause {
                    self.code.write_byte(gvm::JMP);
                    let end_of_true = self.code.get_pointer_position();
                    self.code.write_int(-1);
                    self.code
                        .set(else_pos, self.code.get_pointer_position());
                    self.compile_statement(else_stmt);
                    self.code
                        .set(end_of_true, self.code.get_pointer_position());
                } else {
                    self.code
                        .set(else_pos, self.code.get_pointer_position());
                }
            }
            Statement::While {
                condition,
                body,
                pos,
            } => {
                self.emit_debug(pos);
                let loop_start = self.code.get_pointer_position();
                self.compile_expression(condition);
                self.loop_stack.push(LoopInfo::new());

                self.code.write_byte(gvm::NOT);
                self.code.write_byte(gvm::CJMP);
                let placeholder = self.code.get_pointer_position();
                self.code.write_int(-1);
                self.compile_statement(body);
                self.code.write_byte(gvm::JMP);
                self.code.write_int(loop_start);

                let end_pos = self.code.get_pointer_position();
                self.code.set(placeholder, end_pos);

                let loop_info = self.loop_stack.pop().unwrap();
                for bp in &loop_info.break_positions {
                    self.code.set(*bp, end_pos);
                }
                for cp in &loop_info.continue_positions {
                    self.code.set(*cp, loop_start);
                }
            }
            Statement::For {
                init,
                condition,
                update,
                body,
                pos,
            } => {
                self.emit_debug(pos);
                self.compile_expression(init);
                self.code.write_byte(gvm::POP);
                let condition_pos = self.code.get_pointer_position();
                self.compile_expression(condition);
                self.code.write_byte(gvm::NOT);
                self.code.write_byte(gvm::CJMP);
                let else_pos = self.code.get_pointer_position();
                self.code.write_int(-1);
                self.loop_stack.push(LoopInfo::new());
                self.compile_statement(body);
                let update_pos = self.code.get_pointer_position();
                self.compile_expression(update);
                self.code.write_byte(gvm::POP);
                self.code.write_byte(gvm::JMP);
                self.code.write_int(condition_pos);
                let end_pos = self.code.get_pointer_position();
                self.code.set(else_pos, end_pos);

                let loop_info = self.loop_stack.pop().unwrap();
                for bp in &loop_info.break_positions {
                    self.code.set(*bp, end_pos);
                }
                for cp in &loop_info.continue_positions {
                    self.code.set(*cp, update_pos);
                }
            }
            Statement::Return { value, pos } => {
                self.emit_debug(pos);
                if let Some(ret_val) = value {
                    if self.can_optimize_tail_recursion(ret_val) {
                        self.generate_tail_recursion_check(ret_val);
                        self.compile_expression(ret_val);
                    } else {
                        self.compile_expression(ret_val);
                    }
                } else {
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(0);
                    self.code.write_string("Undefined");
                }
                self.code.write_byte(gvm::RETURN);
            }
            Statement::Throw { exception, pos } => {
                self.emit_debug(pos);
                self.compile_expression(exception);
                self.code.write_byte(gvm::THROW);
            }
            Statement::Break(_pos) => {
                self.code.write_byte(gvm::JMP);
                let jump_pos = self.code.get_pointer_position();
                self.code.write_int(-1);
                if let Some(loop_info) = self.loop_stack.last_mut() {
                    loop_info.break_positions.push(jump_pos);
                }
            }
            Statement::Continue(_pos) => {
                self.code.write_byte(gvm::JMP);
                let jump_pos = self.code.get_pointer_position();
                self.code.write_int(-1);
                if let Some(loop_info) = self.loop_stack.last_mut() {
                    loop_info.continue_positions.push(jump_pos);
                }
            }
            Statement::TryCatch {
                try_block,
                catch_var,
                catch_block,
                pos,
            } => {
                self.emit_debug(pos);
                let start_index = self.code.get_pointer_position();
                self.compile_statement(try_block);
                self.code.write_byte(gvm::JMP);
                let end_of_try = self.code.get_pointer_position();
                self.code.write_int(0);

                // Register catch variable as local
                self.register_local_variable(catch_var);
                let local_idx = self.current_locals.iter().position(|n| n == catch_var).unwrap();

                // LDS to the catch variable slot, then PUT the exception value into it
                self.code.write_byte(gvm::LDS);
                self.code
                    .write_int(1 + self.current_parameters.len() as i32 + local_idx as i32);
                self.code.write_byte(gvm::PUT);

                self.compile_statement(catch_block);
                let current_pos = self.code.get_pointer_position();
                self.code.set(end_of_try, current_pos);

                // Register exception handler on the current function
                let catch_start = end_of_try + 4; // skip the JMP int argument
                self.program
                    .as_mut()
                    .unwrap()
                    .get_function_mut(self.current_function_idx)
                    .unwrap()
                    .register_catch_block(start_index, end_of_try - 1, catch_start);
            }
            Statement::Scope { statements, pos } => {
                self.emit_debug(pos);
                for s in statements {
                    self.compile_statement(s);
                }
            }
            Statement::Void(_pos) => {
                // No-op
            }
        }
    }

    // --- Expression compilation ---

    fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Additive(lhs, op, rhs) => {
                self.compile_expression(lhs);
                self.compile_expression(rhs);
                match op.as_str() {
                    "+" => self.code.write_byte(gvm::ADD),
                    "-" => self.code.write_byte(gvm::SUB),
                    _ => panic!("Unknown additive operator: {}", op),
                }
            }
            Expression::Multiplicative(lhs, op, rhs) => {
                self.compile_expression(lhs);
                self.compile_expression(rhs);
                match op.as_str() {
                    "*" => self.code.write_byte(gvm::MULT),
                    "/" => self.code.write_byte(gvm::DIV),
                    "%" => self.code.write_byte(gvm::MOD),
                    _ => panic!("Unknown multiplicative operator: {}", op),
                }
            }
            Expression::And(lhs, rhs) => {
                self.compile_expression(lhs);
                self.compile_expression(rhs);
                self.code.write_byte(gvm::AND);
            }
            Expression::Or(lhs, rhs) => {
                self.compile_expression(lhs);
                self.compile_expression(rhs);
                self.code.write_byte(gvm::OR);
            }
            Expression::Not(arg) => {
                self.compile_expression(arg);
                self.code.write_byte(gvm::NOT);
            }
            Expression::Equality(lhs, op, rhs) => {
                self.compile_expression(lhs);
                self.compile_expression(rhs);
                match op.as_str() {
                    "==" => {
                        self.code.write_byte(gvm::EQL);
                    }
                    "!=" => {
                        self.code.write_byte(gvm::EQL);
                        self.code.write_byte(gvm::NOT);
                    }
                    _ => panic!("Unknown equality operator: {}", op),
                }
            }
            Expression::Relational(lhs, op, rhs) => {
                match op.as_str() {
                    ">" => {
                        self.compile_expression(lhs);
                        self.compile_expression(rhs);
                        self.code.write_byte(gvm::GT);
                    }
                    "<" => {
                        self.compile_expression(lhs);
                        self.compile_expression(rhs);
                        self.code.write_byte(gvm::LT);
                    }
                    ">=" => {
                        // >= is (lhs > rhs) || (lhs == rhs)
                        let desugared = Expression::Or(
                            Box::new(Expression::Relational(
                                lhs.clone(),
                                ">".to_string(),
                                rhs.clone(),
                            )),
                            Box::new(Expression::Equality(
                                lhs.clone(),
                                "==".to_string(),
                                rhs.clone(),
                            )),
                        );
                        self.compile_expression(&desugared);
                    }
                    "<=" => {
                        // <= is (lhs < rhs) || (lhs == rhs)
                        let desugared = Expression::Or(
                            Box::new(Expression::Relational(
                                lhs.clone(),
                                "<".to_string(),
                                rhs.clone(),
                            )),
                            Box::new(Expression::Equality(
                                lhs.clone(),
                                "==".to_string(),
                                rhs.clone(),
                            )),
                        );
                        self.compile_expression(&desugared);
                    }
                    _ => panic!("Unknown relational operator: {}", op),
                }
            }
            Expression::Assignment {
                value,
                operator,
                variable,
            } => {
                let op = operator.as_deref().unwrap_or("=");
                match op {
                    "=" => {
                        if matches!(value.as_ref(), Expression::FunctionDef { .. }) {
                            if let Expression::Variable { name, field: None } = variable.as_ref() {
                                self.pending_debug_name = Some(name.clone());
                            } else if let Expression::This { field: Some(f) } = variable.as_ref() {
                                if let Expression::Variable { name, field: None } = f.as_ref() {
                                    self.pending_debug_name = Some(name.clone());
                                }
                            }
                        }
                        self.compile_expression(value);
                        self.compile_expression(variable);
                        self.code.write_byte(gvm::PUT);
                    }
                    "+=" => {
                        self.compile_expression(variable);
                        self.compile_expression(value);
                        self.code.write_byte(gvm::ADD);
                        self.compile_expression(variable);
                        self.code.write_byte(gvm::PUT);
                    }
                    "-=" => {
                        self.compile_expression(variable);
                        self.compile_expression(value);
                        self.code.write_byte(gvm::SUB);
                        self.compile_expression(variable);
                        self.code.write_byte(gvm::PUT);
                    }
                    "*=" => {
                        self.compile_expression(variable);
                        self.compile_expression(value);
                        self.code.write_byte(gvm::MULT);
                        self.compile_expression(variable);
                        self.code.write_byte(gvm::PUT);
                    }
                    "/=" => {
                        self.compile_expression(variable);
                        self.compile_expression(value);
                        self.code.write_byte(gvm::DIV);
                        self.compile_expression(variable);
                        self.code.write_byte(gvm::PUT);
                    }
                    _ => panic!("Unsupported assignment operator: {}", op),
                }
            }
            Expression::Constant {
                value,
                type_name,
                string,
            } => {
                if let Some(s) = string {
                    let index = self
                        .program
                        .as_mut()
                        .unwrap()
                        .add_string(s.clone());
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(index);
                    self.code.write_string("String");
                } else if *value == -1 {
                    // Object constructor constant
                    self.code.write_byte(gvm::NEW);
                    self.code.write_string(type_name);
                } else {
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(*value);
                    self.code.write_string(type_name);
                }
            }
            Expression::Variable { name, field } => {
                if self.current_parameters.contains(name) && field.is_none() {
                    // Parameter reference
                    self.code.write_byte(gvm::LDS);
                    let idx = self
                        .current_parameters
                        .iter()
                        .position(|p| p == name)
                        .unwrap();
                    self.code.write_int(1 + idx as i32);
                } else if self.current_locals.contains(name) && field.is_none() {
                    // Local variable reference
                    self.code.write_byte(gvm::LDS);
                    let idx = self
                        .current_locals
                        .iter()
                        .position(|l| l == name)
                        .unwrap();
                    self.code
                        .write_int(1 + self.current_parameters.len() as i32 + idx as i32);
                } else if field.is_some() {
                    // Field access: compile parent, then GET the field
                    self.compile_expression(field.as_ref().unwrap());
                    let str_ref = self
                        .program
                        .as_mut()
                        .unwrap()
                        .add_string(name.clone());
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(str_ref);
                    self.code.write_string("String");
                    self.code.write_byte(gvm::GET);
                } else {
                    // Dynamic variable lookup
                    let str_ref = self
                        .program
                        .as_mut()
                        .unwrap()
                        .add_string(name.clone());
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(str_ref);
                    self.code.write_string("String");
                    self.code.write_byte(gvm::GETDYNAMIC);
                }
            }
            Expression::This { field } => {
                self.code.write_byte(gvm::LDS);
                self.code.write_int(0);
                if let Some(f) = field {
                    self.compile_expression(f);
                }
            }
            Expression::FunctionCall {
                function,
                parameters,
                field,
            } => {
                if field.is_none() {
                    // Add pointer to this
                    self.code.write_byte(gvm::LDS);
                    self.code.write_int(0);
                } else {
                    self.compile_expression(field.as_ref().unwrap());
                }
                for p in parameters {
                    self.compile_expression(p);
                }
                self.compile_expression(function);
                self.code.write_byte(gvm::INVOKE);
                self.code.write_int(parameters.len() as i32);
            }
            Expression::FunctionDef {
                parameters,
                statements,
            } => {
                // Save current function state
                let saved = FunctionState {
                    code: std::mem::replace(&mut self.code, RandomAccessByteStream::new()),
                    function_idx: self.current_function_idx,
                    parameters: std::mem::replace(&mut self.current_parameters, parameters.clone()),
                    locals: std::mem::take(&mut self.current_locals),
                };

                // Create new function
                let function_code = RandomAccessByteStream::new();
                let mut func = GvmFunction::new(function_code, parameters.clone());
                if let Some(name) = self.pending_debug_name.take() {
                    func.set_debug_name(name);
                }
                let index = self.program.as_mut().unwrap().add_function(func);
                self.current_function_idx = index;
                self.program
                    .as_mut()
                    .unwrap()
                    .get_function_mut(index)
                    .unwrap()
                    .set_index(index);

                // Compile function body
                for s in statements {
                    self.compile_statement(s);
                }

                // Add implicit return if last statement is not a return
                let needs_implicit_return = statements
                    .last()
                    .map(|s| !matches!(s, Statement::Return { .. }))
                    .unwrap_or(true);
                if needs_implicit_return {
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(0);
                    self.code.write_string("Undefined");
                    self.code.write_byte(gvm::RETURN);
                }

                // Save the function bytecode
                let func_code = self.code.clone();
                self.program
                    .as_mut()
                    .unwrap()
                    .get_function_mut(index)
                    .unwrap()
                    .set_bytecode(func_code);

                // Also register locals on the function
                for local in &self.current_locals {
                    self.program
                        .as_mut()
                        .unwrap()
                        .get_function_mut(index)
                        .unwrap()
                        .register_local_variable(local.clone());
                }

                // Restore parent function state
                self.code = saved.code;
                self.current_function_idx = saved.function_idx;
                self.current_parameters = saved.parameters;
                self.current_locals = saved.locals;

                // Emit LDC_D with function index
                self.code.write_byte(gvm::LDC_D);
                self.code.write_int(index);
                self.code.write_string("Function");
            }
            Expression::NativeFunctionCall { parameters } => {
                let identifier = self.get_native_method_index(parameters.len() as i32);
                for p in parameters {
                    self.compile_expression(p);
                }
                self.code.write_byte(gvm::LDC_D);
                self.code.write_int(identifier);
                self.code.write_string("Function");
                self.code.write_byte(gvm::NATIVE);
            }
            Expression::ConstructorCall { function } => {
                self.compile_expression(function);
            }
            Expression::ImplicitConstructor { statements } => {
                // Create new object
                self.code.write_byte(gvm::NEW);
                self.code.write_string("Object");

                // Build a function containing statements + return this
                let pos = Position { line: 0, column: 0 };
                let mut func_statements = statements.clone();
                func_statements.push(Statement::Return {
                    value: Some(Expression::This { field: None }),
                    pos,
                });
                let func_def = Expression::FunctionDef {
                    parameters: Vec::new(),
                    statements: func_statements,
                };
                self.compile_expression(&func_def);

                self.code.write_byte(gvm::INVOKE);
                self.code.write_int(0);
            }
            Expression::ArrayDefinition { elements } => {
                self.code.write_byte(gvm::NEW);
                self.code.write_string("Array");
                for (counter, elem) in elements.iter().enumerate() {
                    self.compile_expression(elem);
                    self.code.write_byte(gvm::LDS);
                    self.code.write_int(-1);
                    self.code.write_byte(gvm::LDC_D);
                    self.code.write_int(counter as i32);
                    self.code.write_string("Number");
                    self.code.write_byte(gvm::GET);
                    self.code.write_byte(gvm::PUT);
                    self.code.write_byte(gvm::POP);
                }
            }
            Expression::ArrayReference { index, reference } => {
                self.compile_expression(reference);
                self.compile_expression(index);
                self.code.write_byte(gvm::GET);
            }
            Expression::MapDefinition { keys, values } => {
                self.code.write_byte(gvm::NEW);
                self.code.write_string("Object");
                for i in 0..keys.len() {
                    self.compile_expression(&values[i]);
                    self.code.write_byte(gvm::LDS);
                    self.code.write_int(-1);
                    self.compile_expression(&keys[i]);
                    self.code.write_byte(gvm::GET);
                    self.code.write_byte(gvm::PUT);
                    self.code.write_byte(gvm::POP);
                }
            }
            Expression::Fork => {
                self.code.write_byte(gvm::FORK);
            }
            Expression::PostFixOperator { operator, argument } => {
                match operator.as_str() {
                    "++" => {
                        // Push original value (argument + 0)
                        let add0 = Expression::Additive(
                            argument.clone(),
                            "+".to_string(),
                            Box::new(Expression::Constant {
                                value: 0,
                                type_name: "Number".to_string(),
                                string: None,
                            }),
                        );
                        self.compile_expression(&add0);

                        // Increment: argument = argument + 1
                        let add1 = Expression::Additive(
                            argument.clone(),
                            "+".to_string(),
                            Box::new(Expression::Constant {
                                value: 1,
                                type_name: "Number".to_string(),
                                string: None,
                            }),
                        );
                        let assign = Expression::Assignment {
                            value: Box::new(add1),
                            operator: None,
                            variable: argument.clone(),
                        };
                        self.compile_expression(&assign);
                        self.code.write_byte(gvm::POP);
                    }
                    "--" => {
                        // Push original value (argument + 0)
                        let add0 = Expression::Additive(
                            argument.clone(),
                            "+".to_string(),
                            Box::new(Expression::Constant {
                                value: 0,
                                type_name: "Number".to_string(),
                                string: None,
                            }),
                        );
                        self.compile_expression(&add0);

                        // Decrement: argument = argument - 1
                        let sub1 = Expression::Additive(
                            argument.clone(),
                            "-".to_string(),
                            Box::new(Expression::Constant {
                                value: 1,
                                type_name: "Number".to_string(),
                                string: None,
                            }),
                        );
                        let assign = Expression::Assignment {
                            value: Box::new(sub1),
                            operator: None,
                            variable: argument.clone(),
                        };
                        self.compile_expression(&assign);
                        self.code.write_byte(gvm::POP);
                    }
                    _ => panic!("Unknown postfix operator: {}", operator),
                }
            }
            Expression::Conditional {
                condition,
                positive,
                negative,
            } => {
                self.compile_expression(condition);
                self.code.write_byte(gvm::NOT);
                self.code.write_byte(gvm::CJMP);
                let else_pos = self.code.get_pointer_position();
                self.code.write_int(-1);
                self.compile_expression(positive);
                self.code.write_byte(gvm::JMP);
                let end_of_true = self.code.get_pointer_position();
                self.code.write_int(-1);
                self.code
                    .set(else_pos, self.code.get_pointer_position());
                self.compile_expression(negative);
                self.code
                    .set(end_of_true, self.code.get_pointer_position());
            }
        }
    }

    fn emit_debug(&mut self, pos: &Position) {
        self.code.write_byte(gvm::DEBUG);
        self.code.write_int(pos.line);
        let module_name = self.current_module_name.clone();
        let module_idx = self
            .program
            .as_mut()
            .unwrap()
            .add_string(module_name);
        self.code.write_int(module_idx);
    }

    fn register_local_variable(&mut self, name: &str) {
        if !self.current_locals.contains(&name.to_string()) {
            self.current_locals.push(name.to_string());
        }
    }

    fn can_optimize_tail_recursion(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::FunctionCall { .. })
            && self.is_enabled(&CompilerOptimization::TailRecursion)
    }

    fn generate_tail_recursion_check(&mut self, expr: &Expression) {
        if let Expression::FunctionCall {
            function,
            parameters,
            ..
        } = expr
        {
            // Compile the function reference to compare
            self.compile_expression(function);
            // Load current function index
            self.code.write_byte(gvm::LDC_D);
            self.code.write_int(self.current_function_idx);
            self.code.write_string("Function");
            self.code.write_byte(gvm::EQL);
            self.code.write_byte(gvm::NOT);
            self.code.write_byte(gvm::CJMP);
            let jump_location = self.code.get_pointer_position();
            self.code.write_int(-1);

            // Update parameters in place
            for (x, param) in parameters.iter().enumerate() {
                self.compile_expression(param);
                self.code.write_byte(gvm::LDS);
                self.code.write_int(1 + x as i32);
                self.code.write_byte(gvm::PUT);
                self.code.write_byte(gvm::POP);
            }

            // Jump to beginning of function
            self.code.write_byte(gvm::JMP);
            self.code.write_int(0);
            let jump = self.code.get_pointer_position();
            self.code.set(jump_location, jump);
        }
    }
}

