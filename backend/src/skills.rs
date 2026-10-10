//! Skill dictionary and extraction.
//!
//! Matching is deterministic on purpose: the same CV and job description always give the
//! same skills, and the app can show exactly why a job scored the way it did.

/// (canonical name, aliases, case_sensitive)
/// Case-sensitive entries are words that are also common English ("Go", "Rust" is fine).
const SKILLS: &[(&str, &[&str], bool)] = &[
    ("React Native", &["react native", "react-native", "reactnative"], false),
    ("React", &["react", "react.js", "reactjs"], false),
    ("TypeScript", &["typescript", "ts"], false),
    ("JavaScript", &["javascript", "js", "es6", "ecmascript"], false),
    ("Redux", &["redux", "redux toolkit", "rtk"], false),
    ("GraphQL", &["graphql", "apollo"], false),
    ("REST APIs", &["restful", "rest api", "rest apis", "rest-api"], false),
    ("Node.js", &["node.js", "nodejs"], false),
    ("Express", &["express.js", "expressjs"], false),
    ("Next.js", &["next.js", "nextjs"], false),
    ("Vue", &["vue", "vue.js", "vuejs"], false),
    ("Angular", &["angular"], false),
    ("HTML", &["html", "html5"], false),
    ("CSS", &["css", "css3", "tailwind", "sass", "scss"], false),
    ("Rust", &["rust"], false),
    ("Go", &["Go", "Golang", "golang"], true),
    ("Python", &["python"], false),
    ("Django", &["django"], false),
    ("FastAPI", &["fastapi"], false),
    ("Java", &["java"], false),
    ("Spring Boot", &["spring boot", "spring"], false),
    ("Kotlin", &["kotlin"], false),
    ("Swift", &["swift", "swiftui"], false),
    ("Objective-C", &["objective-c", "objc"], false),
    ("Android", &["android"], false),
    ("iOS", &["ios"], false),
    ("Flutter", &["flutter", "dart"], false),
    ("Expo", &["expo"], false),
    ("C++", &["c++", "cpp"], false),
    ("C#", &["c#", ".net", "dotnet"], false),
    ("PHP", &["php", "laravel"], false),
    ("Ruby", &["ruby", "rails", "ruby on rails"], false),
    ("SQL", &["sql"], false),
    ("PostgreSQL", &["postgresql", "postgres"], false),
    ("MySQL", &["mysql"], false),
    ("MongoDB", &["mongodb", "mongo"], false),
    ("Redis", &["redis"], false),
    ("Elasticsearch", &["elasticsearch", "opensearch"], false),
    ("Kafka", &["kafka"], false),
    ("RabbitMQ", &["rabbitmq"], false),
    ("gRPC", &["grpc"], false),
    ("Microservices", &["microservices", "microservice"], false),
    ("AWS", &["aws", "amazon web services", "lambda", "ec2", "s3"], false),
    ("GCP", &["gcp", "google cloud"], false),
    ("Azure", &["azure"], false),
    ("Docker", &["docker", "containers"], false),
    ("Kubernetes", &["kubernetes", "k8s"], false),
    ("Terraform", &["terraform"], false),
    ("CI/CD", &["ci/cd", "ci cd", "github actions", "jenkins", "gitlab ci", "continuous integration"], false),
    ("Git", &["git", "github", "gitlab"], false),
    ("Jest", &["jest"], false),
    ("Detox", &["detox"], false),
    ("Cypress", &["cypress"], false),
    ("Playwright", &["playwright"], false),
    ("Unit testing", &["unit testing", "unit tests", "tdd"], false),
    ("Firebase", &["firebase"], false),
    ("WebSockets", &["websocket", "websockets", "socket.io"], false),
    ("Payments", &["payments", "payment", "checkout", "upi", "stripe", "razorpay"], false),
    ("Performance optimization", &["performance optimization", "performance tuning", "app performance"], false),
    ("System design", &["system design", "distributed systems", "scalable systems"], false),
    ("Machine learning", &["machine learning", "ml", "deep learning"], false),
    ("LLMs", &["llm", "llms", "large language models", "genai", "generative ai"], false),
    ("Figma", &["figma"], false),
    ("Agile", &["agile", "scrum"], false),
];

/// Canonical names of every skill found in `text`, in dictionary order, without duplicates.
pub fn extract_skills(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut found = Vec::new();
    for (canonical, aliases, case_sensitive) in SKILLS {
        let hit = aliases.iter().any(|alias| {
            if *case_sensitive {
                contains_term(text, alias)
            } else {
                contains_term(&lower, &alias.to_lowercase())
            }
        });
        if hit {
            found.push((*canonical).to_string());
        }
    }
    // "React" is implied by "React Native" text, so only keep it if it appears on its own.
    if found.iter().any(|s| s == "React Native") && found.iter().any(|s| s == "React") {
        let without_rn = lower.replace("react native", "").replace("react-native", "");
        if !contains_term(&without_rn, "react") && !contains_term(&without_rn, "react.js") && !contains_term(&without_rn, "reactjs") {
            found.retain(|s| s != "React");
        }
    }
    found
}

/// Map a free-typed skill ("reactjs", "Postgres") to its canonical name, or keep it as typed.
pub fn canonicalize(skill: &str) -> String {
    let lower = skill.trim().to_lowercase();
    for (canonical, aliases, _) in SKILLS {
        if canonical.to_lowercase() == lower || aliases.iter().any(|a| a.to_lowercase() == lower) {
            return (*canonical).to_string();
        }
    }
    skill.trim().to_string()
}

/// True if `term` appears in `haystack` with no letter or digit directly on either side.
fn contains_term(haystack: &str, term: &str) -> bool {
    if term.is_empty() {
        return false;
    }
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(term) {
        let begin = start + pos;
        let end = begin + term.len();
        let before_ok = haystack[..begin].chars().next_back().map_or(true, |c| !is_word_char(c));
        let after_ok = haystack[end..].chars().next().map_or(true, |c| !is_word_char(c));
        if before_ok && after_ok {
            return true;
        }
        start = begin + term.chars().next().map_or(1, |c| c.len_utf8());
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '+' || c == '#'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_common_stack() {
        let s = extract_skills("Senior engineer: React Native, TypeScript, Redux Toolkit and GraphQL. Postgres a plus.");
        assert_eq!(s, vec!["React Native", "TypeScript", "Redux", "GraphQL", "PostgreSQL"]);
    }

    #[test]
    fn go_needs_capital_or_golang() {
        assert!(!extract_skills("you will go above and beyond").contains(&"Go".to_string()));
        assert!(extract_skills("Backend in Go and Rust").contains(&"Go".to_string()));
        assert!(extract_skills("golang services").contains(&"Go".to_string()));
    }

    #[test]
    fn word_boundaries() {
        // "java" must not match inside "javascript"
        let s = extract_skills("Strong JavaScript skills");
        assert!(s.contains(&"JavaScript".to_string()));
        assert!(!s.contains(&"Java".to_string()));
        // "react" must not match inside "reactive"
        assert!(!extract_skills("reactive programming").contains(&"React".to_string()));
    }

    #[test]
    fn react_native_does_not_imply_react() {
        let s = extract_skills("React Native developer");
        assert!(!s.contains(&"React".to_string()));
        let s = extract_skills("React Native and React web apps");
        assert!(s.contains(&"React".to_string()));
    }

    #[test]
    fn canonicalizes_aliases() {
        assert_eq!(canonicalize("postgres"), "PostgreSQL");
        assert_eq!(canonicalize("ReactJS"), "React");
        assert_eq!(canonicalize("Some niche tool"), "Some niche tool");
    }
}
