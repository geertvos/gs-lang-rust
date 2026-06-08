use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;

use gs_core::bridge::NativeMethodWrapper;
use gs_core::bridge::registry::NativeRegistry;
use gs_core::core::gvm::Gvm;
use gs_core::core::types::BooleanType;
use gs_core::debug::DebugInfo;
use gs_core::program::{GvmProgram, GvmProgramSerializer, NativeMethodFactory};

use gs_lang::ast;
use gs_lang::compiler::GScriptCompiler;
use gs_lang::lang::exception_handler::GscriptExceptionHandler;
use gs_lang::lang::types::array_type::ArrayType;
use gs_lang::lang::types::number::NumberType;
use gs_lang::lang::types::object_type::ObjectType;
use gs_lang::lang::types::native_object_type::NativeObjectType;
use gs_lang::lang::types::string_type::StringType;
use gs_lang::lang::value_converter::GscriptValueConverter;
use gs_lang::parser::Parser;
use gs_lang::runtime::NativeStaticMethodWrapper;
use gs_lang::stdlib;

fn create_registry() -> Arc<NativeRegistry> {
    let mut registry = NativeRegistry::new();
    stdlib::register_all(&mut registry);
    Arc::new(registry)
}

struct GscriptNativeMethodFactory {
    registry: Arc<NativeRegistry>,
}

impl NativeMethodFactory for GscriptNativeMethodFactory {
    fn create(&self, argument_count: i32) -> Box<dyn NativeMethodWrapper> {
        Box::new(NativeStaticMethodWrapper::new(argument_count, self.registry.clone()))
    }
}

fn register_runtime_types(program: &mut GvmProgram) {
    program.register_type(Box::new(ObjectType));
    program.register_type(Box::new(StringType));
    program.register_type(Box::new(NumberType));
    program.register_type(Box::new(BooleanType));
    program.register_type(Box::new(ArrayType));
    program.register_type(Box::new(NativeObjectType));
}

fn parse_source(path: &Path) -> ast::Module {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let mut parser = Parser::new(&source);
    parser
        .parse_program()
        .unwrap_or_else(|e| panic!("Parse error in {}: {}", path.display(), e))
}

fn find_gslib_dir(source_path: &Path) -> Option<std::path::PathBuf> {
    for var in &["GVM_HOME_RUST", "GVM_HOME"] {
        if let Ok(home) = std::env::var(var) {
            let p = std::path::PathBuf::from(&home).join("gslib");
            if p.is_dir() {
                return Some(p);
            }
        }
    }
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let candidates = [
        exe_dir.join("gslib"),
        source_path
            .parent()
            .unwrap_or(Path::new("."))
            .join("gslib"),
        std::path::PathBuf::from("gslib"),
    ];
    candidates.into_iter().find(|p| p.is_dir())
}

fn compile_modules(source_path: &Path, registry: Arc<NativeRegistry>) -> GvmProgram {
    use std::collections::{HashSet, VecDeque};

    let main_module = parse_source(source_path);
    let parent_dir = source_path.parent().unwrap_or(Path::new("."));
    let gslib_dir = find_gslib_dir(source_path);

    let mut parsed_modules = Vec::new();
    let mut loaded: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = main_module.imports.iter().cloned().collect();

    while let Some(module_name) = queue.pop_front() {
        if loaded.contains(&module_name) {
            continue;
        }
        loaded.insert(module_name.clone());

        let import_path = parent_dir.join(format!("{}.gs", module_name));
        let resolved_path = if import_path.exists() {
            import_path
        } else if let Some(ref gslib) = gslib_dir {
            let gslib_path = gslib.join(format!("{}.gs", module_name));
            if gslib_path.exists() {
                gslib_path
            } else {
                panic!(
                    "Module '{}' not found in {} or gslib/",
                    module_name,
                    parent_dir.display()
                );
            }
        } else {
            panic!(
                "Module '{}' not found: {}",
                module_name,
                import_path.display()
            );
        };

        let import_module = parse_source(&resolved_path);
        for imp in &import_module.imports {
            if !loaded.contains(imp) {
                queue.push_back(imp.clone());
            }
        }
        parsed_modules.insert(0, import_module);
    }

    parsed_modules.push(main_module);

    let mut compiler = GScriptCompiler::new(registry);
    compiler.compile_modules(parsed_modules)
}

fn run_program(program: GvmProgram, debug: bool) {
    let mut vm = Gvm::new(program);
    if debug {
        vm.set_debug(true);
    }
    vm.run();
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  gs-lang <file.gs>                  Compile and run");
    eprintln!("  gs-lang --debug <file.gs>          Compile and run with debug tracing");
    eprintln!("  gs-lang --compile <file.gs> -o <file.gsc>  Compile to binary");
    eprintln!("  gs-lang --run <file.gsc>           Load and run binary");
    eprintln!("  gs-lang --debug --run <file.gsc>   Load and run binary with debug tracing");
    eprintln!("  gs-lang --asm <file.gs>            Compile and print assembly");
    eprintln!("  gs-lang --asm --run <file.gsc>     Load binary and print assembly");
}

fn main() {
    let all_args: Vec<String> = std::env::args().skip(1).collect();
    let debug = all_args.iter().any(|a| a == "--debug");
    let asm = all_args.iter().any(|a| a == "--asm");
    let args: Vec<String> = all_args.into_iter().filter(|a| a != "--debug" && a != "--asm").collect();

    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let registry = create_registry();

    if args[0] == "--compile" {
        // gs-lang --compile <file.gs> -o <file.gsc>
        if args.len() < 4 || args[2] != "-o" {
            eprintln!("Error: expected --compile <file.gs> -o <file.gsc>");
            print_usage();
            std::process::exit(1);
        }
        let source_path = Path::new(&args[1]);
        let output_path = Path::new(&args[3]);

        let mut program = compile_modules(source_path, registry);
        let file = fs::File::create(output_path)
            .unwrap_or_else(|e| panic!("Failed to create {}: {}", output_path.display(), e));
        let mut writer = BufWriter::new(file);
        GvmProgramSerializer::write_to(&mut program, &mut writer)
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", output_path.display(), e));

        eprintln!("Compiled {} -> {}", source_path.display(), output_path.display());
    } else if args[0] == "--run" {
        // gs-lang --run <file.gsc>
        if args.len() < 2 {
            eprintln!("Error: expected --run <file.gsc>");
            print_usage();
            std::process::exit(1);
        }
        let binary_path = Path::new(&args[1]);

        let file = fs::File::open(binary_path)
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", binary_path.display(), e));
        let mut reader = BufReader::new(file);
        let factory = GscriptNativeMethodFactory { registry };
        let mut program = GvmProgramSerializer::read_from(
            &mut reader,
            Box::new(GscriptExceptionHandler),
            Box::new(GscriptValueConverter),
            &factory,
        )
        .unwrap_or_else(|e| panic!("Failed to load {}: {}", binary_path.display(), e));

        register_runtime_types(&mut program);
        if asm {
            DebugInfo::disassemble(&mut std::io::stdout(), &program);
        } else {
            run_program(program, debug);
        }
    } else {
        let path = Path::new(&args[0]);
        if path.extension().and_then(|e| e.to_str()) == Some("gsc") {
            let file = fs::File::open(path)
                .unwrap_or_else(|e| panic!("Failed to open {}: {}", path.display(), e));
            let mut reader = BufReader::new(file);
            let factory = GscriptNativeMethodFactory { registry };
            let mut program = GvmProgramSerializer::read_from(
                &mut reader,
                Box::new(GscriptExceptionHandler),
                Box::new(GscriptValueConverter),
                &factory,
            )
            .unwrap_or_else(|e| panic!("Failed to load {}: {}", path.display(), e));

            register_runtime_types(&mut program);
            if asm {
                DebugInfo::disassemble(&mut std::io::stdout(), &program);
            } else {
                run_program(program, debug);
            }
        } else {
            let program = compile_modules(path, registry);
            if asm {
                DebugInfo::disassemble(&mut std::io::stdout(), &program);
            } else {
                run_program(program, debug);
            }
        }
    }
}
