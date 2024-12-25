#[derive(Debug)]  
pub struct TelegramMarkdownConverter {  
    special_chars: Vec<char>,  
}  
  
impl TelegramMarkdownConverter {  
    pub fn new() -> Self {  
        TelegramMarkdownConverter {  
            special_chars: vec![  
                '[', ']', '(', ')', '.', '!', '#', '+',   
                '-', '=', '{', '}', '>', '<', '&'  
            ],  
        }  
    }  
  
    pub fn convert(&self, text: &str) -> String {  
        let mut result = String::new();  
        let mut in_code_block = false;  
  
        let mut chars = text.chars().peekable();  
        while let Some(c) = chars.next() {  
            match c {  
                '`' => {  
                    if chars.peek() == Some(&'`') && chars.nth(1) == Some('`') {  
                        if !in_code_block {  
                            result.push_str("```");  
                            in_code_block = true;  
                        } else {  
                            result.push_str("```");  
                            in_code_block = false;  
                        }  
                    } else {  
                        result.push('`');  
                    }  
                },  
                '*' | '_' | '~' | '|' => {  
                    if in_code_block {  
                        result.push(c);  
                    } else {  
                        result.push(c);  
                    }  
                },  
                '\\' => {  
                    result.push_str("\\\\");  
                },  
                _ => {  
                    if in_code_block {  
                        result.push(c);  
                    } else if self.special_chars.contains(&c) {  
                        result.push('\\');  
                        result.push(c);  
                    } else {  
                        result.push(c);  
                    }  
                }  
            }  
        }  
        result  
    }  
}
