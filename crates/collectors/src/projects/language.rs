//! Linguagem principal pelos arquivos na raiz do projeto.

/// Primeiro marcador encontrado, na ordem de especificidade (TypeScript antes de JavaScript).
const MARKERS: &[(&str, &str)] = &[
    ("Cargo.toml", "Rust"),
    ("tsconfig.json", "TypeScript"),
    ("package.json", "JavaScript"),
    ("pyproject.toml", "Python"),
    ("requirements.txt", "Python"),
    ("go.mod", "Go"),
    ("build.gradle.kts", "Kotlin"),
    ("pom.xml", "Java"),
    ("build.gradle", "Java"),
    ("composer.json", "PHP"),
    ("Gemfile", "Ruby"),
    ("pubspec.yaml", "Dart"),
    ("mix.exs", "Elixir"),
    ("CMakeLists.txt", "C++"),
];

pub(super) fn detect<'a>(names: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let names: Vec<&str> = names.into_iter().collect();
    if let Some((_, language)) = MARKERS.iter().find(|(file, _)| names.contains(file)) {
        return Some((*language).to_owned());
    }
    let has_ext = |ext: &str| names.iter().any(|n| n.ends_with(ext));
    if has_ext(".csproj") || has_ext(".sln") {
        return Some("C#".to_owned());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marcadores() {
        assert_eq!(detect(["README.md", "Cargo.toml"]).as_deref(), Some("Rust"));
        assert_eq!(
            detect(["package.json", "tsconfig.json"]).as_deref(),
            Some("TypeScript")
        );
        assert_eq!(detect(["App.csproj"]).as_deref(), Some("C#"));
        assert_eq!(detect(["notas.md"]), None);
    }
}
