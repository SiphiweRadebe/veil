use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn analyze() -> Result<()> {
    let cwd = std::env::current_dir()?;
    println!("{} {}", "veil".purple().bold(), "analyze".white());
    println!();

    // Run auditor checks first
    crate::engines::auditor::audit()?;

    // Code metrics
    println!("{} code metrics:\n", "veil".purple().bold());

    let metrics = calculate_metrics(&cwd)?;
    println!("  {} {} files", "files:".dimmed(), metrics.total_files.to_string().cyan());
    println!(
        "  {} {} lines",
        "lines:".dimmed(),
        metrics.total_lines.to_string().cyan()
    );
    println!(
        "  {} {}",
        "complexity:".dimmed(),
        format_complexity(metrics.estimated_complexity).cyan()
    );

    // Detect dead code patterns
    println!("\n{} potential issues:\n", "veil".purple().bold());

    let issues = detect_dead_code(&cwd)?;
    if issues.is_empty() {
        println!("  {} no obvious dead code detected", "✓".green());
    } else {
        for issue in issues.iter().take(5) {
            println!("  {} {} — {}", "⚠".yellow(), issue.file.cyan(), issue.description.dimmed());
        }
        if issues.len() > 5 {
            println!("  {} {} more issues", "...".dimmed(), (issues.len() - 5).to_string().cyan());
        }
    }

    println!();
    Ok(())
}

pub fn deps_visual() -> Result<()> {
    let cwd = std::env::current_dir()?;
    println!("{} {} dependency tree:\n", "veil".purple().bold(), "project".white());

    let deps = extract_dependencies(&cwd)?;

    if deps.is_empty() {
        println!("  {} no dependencies detected", "✓".green());
        return Ok(());
    }

    for (name, count) in deps.iter().take(10) {
        let bar = "█".repeat((*count).min(20));
        println!("  {} {} ({}×)", bar.cyan(), name.white(), count.to_string().dimmed());
    }

    if deps.len() > 10 {
        println!("  {} {} more...", "...".dimmed(), (deps.len() - 10).to_string().cyan());
    }

    println!();
    Ok(())
}

pub fn deps_json() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let deps = extract_dependencies(&cwd)?;

    let json = serde_json::json!({
        "dependencies": deps,
        "total": deps.len(),
    });

    println!("{}", serde_json::to_string_pretty(&json)?);
    Ok(())
}

pub fn impact(file: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    println!(
        "{} {} impact analysis for {}:\n",
        "veil".purple().bold(),
        "checking".white(),
        file.cyan()
    );

    let dependents = find_dependents(&cwd, file)?;

    if dependents.is_empty() {
        println!("  {} no files depend on {}", "✓".green(), file.cyan());
        return Ok(());
    }

    println!("  {} files would be affected:\n", dependents.len().to_string().yellow());

    for dep in dependents.iter().take(10) {
        println!("    {} → {}", "•".dimmed(), dep.cyan());
    }

    if dependents.len() > 10 {
        println!("    {} {} more files", "...".dimmed(), (dependents.len() - 10).to_string().cyan());
    }

    println!();
    Ok(())
}

struct Metrics {
    total_files: usize,
    total_lines: usize,
    estimated_complexity: f32,
}

fn calculate_metrics(path: &Path) -> Result<Metrics> {
    let mut total_files = 0;
    let mut total_lines = 0;
    let mut estimated_complexity = 0.0;

    for entry in walkdir(path, 0) {
        let path = entry.path();
        if is_code_file(&path) {
            total_files += 1;

            if let Ok(content) = fs::read_to_string(&path) {
                let lines = content.lines().count();
                total_lines += lines;

                let complexity_indicators = [
                    ("if", content.matches("if").count()),
                    ("loop", content.matches("for").count() + content.matches("while").count()),
                    ("match", content.matches("match").count()),
                    ("fn", content.matches("fn").count()),
                ];

                for (_, count) in &complexity_indicators {
                    estimated_complexity += *count as f32 * 0.5;
                }
            }
        }
    }

    Ok(Metrics {
        total_files,
        total_lines,
        estimated_complexity: (estimated_complexity / total_files.max(1) as f32).min(10.0),
    })
}

#[derive(Debug)]
struct CodeIssue {
    file: String,
    description: String,
}

fn detect_dead_code(path: &Path) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();

    for entry in walkdir(path, 0) {
        let file_path = entry.path();
        if is_code_file(&file_path) {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if content.contains("let _ =") || content.contains("_unused") {
                    issues.push(CodeIssue {
                        file: file_path.display().to_string(),
                        description: "unused variables detected".to_string(),
                    });
                }

                let comment_lines = content
                    .lines()
                    .filter(|l| l.trim().starts_with("//") || l.trim().starts_with("#"))
                    .count();
                let total_lines = content.lines().count();
                if total_lines > 0 && comment_lines > total_lines / 10 {
                    issues.push(CodeIssue {
                        file: file_path.display().to_string(),
                        description: format!("{}% commented code", (comment_lines * 100) / total_lines),
                    });
                }
            }
        }
    }

    Ok(issues)
}

fn extract_dependencies(path: &Path) -> Result<Vec<(String, usize)>> {
    let mut deps: HashMap<String, usize> = HashMap::new();

    for entry in walkdir(path, 0) {
        let file_path = entry.path();
        if let Ok(content) = fs::read_to_string(&file_path) {
            for line in content.lines() {
                if line.contains("import ") || line.contains("require(") {
                    if let Some(dep) = extract_dep_name(line) {
                        *deps.entry(dep).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    let mut sorted: Vec<_> = deps.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(sorted)
}

fn find_dependents(path: &Path, target_file: &str) -> Result<Vec<String>> {
    let mut dependents = Vec::new();

    for entry in walkdir(path, 0) {
        let file_path = entry.path();
        if let Ok(content) = fs::read_to_string(&file_path) {
            if content.contains(target_file) || content.contains(path_to_module(target_file).as_str()) {
                dependents.push(file_path.display().to_string());
            }
        }
    }

    Ok(dependents)
}

fn is_code_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext_str = ext.to_string_lossy().to_string();
        matches!(
            ext_str.as_str(),
            "rs" | "py" | "js" | "ts" | "go" | "java" | "cpp" | "c" | "h"
        )
    } else {
        false
    }
}

fn walkdir(path: &Path, depth: u32) -> Vec<fs::DirEntry> {
    let mut result = Vec::new();
    let ignore_dirs = ["target", "node_modules", ".git", "dist", "build"];

    if depth > 5 {
        return result;
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if !ignore_dirs.contains(&name.to_string_lossy().as_ref()) {
                if entry.path().is_dir() && depth < 3 {
                    result.extend(walkdir(&entry.path(), depth + 1));
                }
                result.push(entry);
            }
        }
    }

    result
}

fn extract_dep_name(line: &str) -> Option<String> {
    if line.contains("import ") {
        line.split("import ")
            .nth(1)
            .and_then(|s| s.split(|c: char| c.is_whitespace() || c == ';').next())
            .map(|s| s.trim_matches(|c: char| c == '\'' || c == '"').to_string())
    } else if line.contains("require(") {
        line.split("require(")
            .nth(1)
            .and_then(|s| s.split(')').next())
            .map(|s| s.trim_matches(|c: char| c == '\'' || c == '"' || c == '(' || c == ')').to_string())
    } else {
        None
    }
}

fn path_to_module(path: &str) -> String {
    path.replace('/', ".")
        .replace('\\', ".")
        .replace(".rs", "")
        .replace(".py", "")
}

fn format_complexity(value: f32) -> String {
    match value {
        v if v < 1.0 => "simple".to_string(),
        v if v < 3.0 => "moderate".to_string(),
        v if v < 6.0 => "complex".to_string(),
        _ => "very complex".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_code_file() {
        assert!(is_code_file(Path::new("test.rs")));
        assert!(is_code_file(Path::new("test.py")));
        assert!(!is_code_file(Path::new("test.txt")));
    }

    #[test]
    fn test_format_complexity() {
        assert_eq!(format_complexity(0.5), "simple");
        assert_eq!(format_complexity(2.0), "moderate");
        assert_eq!(format_complexity(5.0), "complex");
    }

    #[test]
    fn test_path_to_module() {
        assert_eq!(path_to_module("src/utils.rs"), "src.utils");
    }
}
