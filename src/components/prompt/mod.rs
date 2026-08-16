use crate::schema::Message;

pub type PromptResult<T> = Result<T, PromptError>;

#[derive(Debug, thiserror::Error)]
pub enum PromptError {
    #[error("template error: {0}")]
    Template(String),
    #[error("render error: {0}")]
    Render(String),
}

pub trait ChatTemplate: Send + Sync {
    fn format(&self, messages: Vec<Message>) -> PromptResult<String>;

    fn format_messages(&self, messages: Vec<Message>) -> PromptResult<Vec<Message>>;

    fn get_template(&self) -> &str;

    fn get_variables(&self) -> Vec<String>;
}

pub struct PromptTemplate {
    template: String,
    variables: Vec<String>,
}

impl PromptTemplate {
    pub fn new(template: impl Into<String>) -> Self {
        let template = template.into();
        let variables = Self::extract_variables(&template);
        Self {
            template,
            variables,
        }
    }

    fn extract_variables(template: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut in_brace = false;
        let mut current = String::new();

        for c in template.chars() {
            match c {
                '{' => {
                    in_brace = true;
                    current.clear();
                }
                '}' if in_brace => {
                    if !current.is_empty() {
                        vars.push(current.clone());
                    }
                    in_brace = false;
                    current.clear();
                }
                c if in_brace => {
                    current.push(c);
                }
                _ => {}
            }
        }
        vars
    }

    pub fn invoke(&self, vars: std::collections::HashMap<String, String>) -> PromptResult<String> {
        let mut result = self.template.clone();
        for (key, value) in vars {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, &value);
        }
        Ok(result)
    }
}

impl ChatTemplate for PromptTemplate {
    fn format(&self, messages: Vec<Message>) -> PromptResult<String> {
        let system = messages
            .iter()
            .find(|m| matches!(m.role, crate::schema::RoleType::System));

        let mut parts = Vec::new();

        if let Some(sys_msg) = system {
            if let crate::schema::Content::Text(ref text) = sys_msg.content {
                parts.push(text.clone());
            }
        }

        for msg in messages
            .iter()
            .filter(|m| !matches!(m.role, crate::schema::RoleType::System))
        {
            let role = match msg.role {
                crate::schema::RoleType::User => "user",
                crate::schema::RoleType::Assistant => "assistant",
                crate::schema::RoleType::Tool => "tool",
                crate::schema::RoleType::System => continue,
            };

            let content = match &msg.content {
                crate::schema::Content::Text(s) => s.clone(),
                _ => continue,
            };

            parts.push(format!("{}: {}", role, content));
        }

        Ok(parts.join("\n"))
    }

    fn format_messages(&self, messages: Vec<Message>) -> PromptResult<Vec<Message>> {
        Ok(messages)
    }

    fn get_template(&self) -> &str {
        &self.template
    }

    fn get_variables(&self) -> Vec<String> {
        self.variables.clone()
    }
}

pub fn prompt(template: impl Into<String>) -> PromptTemplate {
    PromptTemplate::new(template)
}
