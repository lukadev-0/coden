pub struct Language {
    pub source_file: &'static str,
    pub compile: Option<&'static [&'static str]>,
    pub execute: &'static [&'static str],
}

pub fn get_by_id(id: &str) -> Option<&'static Language> {
    Some(match id.to_lowercase().as_str() {
        "sh" => &SH,
        "c" => &C,
        "c++" | "cpp" | "cxx" => &CPP,
        "python" | "py" => &PYTHON,
        "rust" | "rs" => &RUST,
        _ => return None,
    })
}

pub const SH: Language = Language {
    source_file: "program.sh",
    compile: None,
    execute: &["sh", "./program.sh"],
};

pub const C: Language = Language {
    source_file: "program.c",
    compile: Some(&["gcc", "-o", "program", "program.c"]),
    execute: &["./program"],
};

pub const CPP: Language = Language {
    source_file: "program.cpp",
    compile: Some(&["g++", "-std=c++26", "-o", "program", "program.cpp"]),
    execute: &["./program"],
};

pub const PYTHON: Language = Language {
    source_file: "program.py",
    compile: None,
    execute: &["python", "program.py"],
};

pub const RUST: Language = Language {
    source_file: "program.rs",
    compile: Some(&["rustc", "-o", "program", "program.rs"]),
    execute: &["./program"],
};
