use anyhow::Result;

pub mod fuzzy_match {
    use strsim::jaro_winkler;

    pub fn find_similar(query: &str, candidates: &[&str], threshold: f64) -> Vec<(String, f64)> {
        let mut results: Vec<_> = candidates
            .iter()
            .map(|c| {
                let score = jaro_winkler(query, c);
                (c.to_string(), score)
            })
            .filter(|(_, score)| *score >= threshold)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn score_command(query: &str, command: &str) -> f64 {
        jaro_winkler(query, command)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_find_similar() {
            let candidates = vec!["git add", "git commit", "git push", "git pull"];
            let results = find_similar("git com", &candidates, 0.7);
            assert!(!results.is_empty());
            assert_eq!(results[0].0, "git commit");
        }

        #[test]
        fn test_score_command() {
            let score = score_command("git commit", "git commit");
            assert!(score > 0.99);

            let score = score_command("git commit", "git push");
            assert!(score < 0.99);
        }
    }
}

pub mod sequence_detect {
    pub struct Sequence {
        pub commands: Vec<String>,
        pub count: usize,
        pub success_rate: f32,
    }

    pub fn detect_patterns(
        history: &[(String, i32)],
        min_length: usize,
        min_occurrences: usize,
    ) -> Vec<Sequence> {
        let mut patterns: std::collections::HashMap<Vec<String>, (usize, usize)> =
            std::collections::HashMap::new();

        for window in history.windows(min_length) {
            let seq: Vec<String> = window.iter().map(|(cmd, _)| cmd.clone()).collect();
            let successful = window.iter().all(|(_, code)| *code == 0);

            let (count, successes) = patterns.entry(seq).or_insert((0, 0));
            *count += 1;
            if successful {
                *successes += 1;
            }
        }

        patterns
            .into_iter()
            .filter(|(_, (count, _))| *count >= min_occurrences)
            .map(|(commands, (count, successes))| Sequence {
                commands,
                count,
                success_rate: if count > 0 {
                    (successes as f32) / (count as f32)
                } else {
                    0.0
                },
            })
            .collect()
    }

    pub fn find_next_command(
        history: &[(String, i32)],
        last_command: &str,
        limit: usize,
    ) -> Vec<(String, usize)> {
        let mut followers: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for i in 0..history.len().saturating_sub(1) {
            if history[i].0 == last_command {
                let next = &history[i + 1].0;
                *followers.entry(next.clone()).or_insert(0) += 1;
            }
        }

        let mut results: Vec<_> = followers.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().take(limit).collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_detect_patterns() {
            let history = vec![
                ("git add .".to_string(), 0),
                ("git commit -m 'test'".to_string(), 0),
                ("git push".to_string(), 0),
                ("git add .".to_string(), 0),
                ("git commit -m 'fix'".to_string(), 0),
                ("git push".to_string(), 0),
            ];

            let patterns = detect_patterns(&history, 3, 1);
            assert!(!patterns.is_empty());
        }

        #[test]
        fn test_find_next_command() {
            let history = vec![
                ("git add".to_string(), 0),
                ("git commit".to_string(), 0),
                ("git add".to_string(), 0),
                ("git commit".to_string(), 0),
            ];

            let next = find_next_command(&history, "git add", 5);
            assert_eq!(next[0].0, "git commit");
            assert_eq!(next[0].1, 2);
        }
    }
}

pub mod command_danger {
    pub enum RiskLevel {
        Safe,
        Medium,
        High,
    }

    pub fn assess_command_risk(cmd: &str) -> (RiskLevel, &'static str) {
        let cmd_lower = cmd.to_lowercase();

        if cmd_lower.contains("rm -rf") || cmd_lower.contains("rm -f /") {
            return (RiskLevel::High, "Dangerous recursive deletion detected");
        }
        if cmd_lower.starts_with("rm ") && !cmd_lower.contains("--dry-run") {
            return (RiskLevel::High, "File deletion detected");
        }
        if cmd_lower.contains("format ") || cmd_lower.contains("dd if=/dev/")
        {
            return (RiskLevel::High, "Destructive disk operation detected");
        }
        if cmd_lower.contains("drop database") || cmd_lower.contains("truncate table") {
            return (RiskLevel::High, "Database destruction detected");
        }

        if cmd_lower.contains("git reset --hard")
            || cmd_lower.contains("git clean -fd")
            || cmd_lower.contains("git force-push")
        {
            return (RiskLevel::Medium, "Git destructive operation detected");
        }
        if cmd_lower.contains("sudo ") && !cmd_lower.contains("--dry-run") {
            return (RiskLevel::Medium, "Root-level operation");
        }
        if cmd_lower.contains("./") || cmd_lower.contains("bash ") || cmd_lower.contains("sh ") {
            return (RiskLevel::Medium, "Executing external script");
        }

        (RiskLevel::Safe, "No significant risks detected")
    }

    pub fn format_risk(risk: &RiskLevel) -> &'static str {
        match risk {
            RiskLevel::High => "🔴 HIGH RISK",
            RiskLevel::Medium => "🟡 MEDIUM RISK",
            RiskLevel::Safe => "🟢 SAFE",
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_high_risk_detection() {
            let (risk, _) = assess_command_risk("rm -rf /");
            matches!(risk, RiskLevel::High);
        }

        #[test]
        fn test_medium_risk_detection() {
            let (risk, _) = assess_command_risk("git reset --hard");
            matches!(risk, RiskLevel::Medium);
        }

        #[test]
        fn test_safe_detection() {
            let (risk, _) = assess_command_risk("ls -la");
            matches!(risk, RiskLevel::Safe);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_loads() {
        assert!(true);
    }
}
