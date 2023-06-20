pub fn rust_to_lisp_symbol(name: &str) -> String {
    let name = str::replace(&name, "_star", "*");
    let name = str::replace(&name, "_bang", "!");
    let name = str::replace(&name, "_quest", "?");
    let name = str::replace(&name, "_", "-");
    name
}
