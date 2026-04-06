#[derive(Debug, Clone)]
pub struct ModelPrice {
    pub input_per_1m: f64,
    pub output_per_1m: f64,
}

pub fn model_price(model: &str) -> Option<ModelPrice> {
    let m = model.to_lowercase();
    if m.contains("claude-sonnet-4") || m.contains("claude-4-sonnet") {
        return Some(ModelPrice {
            input_per_1m: 3.00,
            output_per_1m: 15.00,
        });
    }
    if m.contains("claude-3-5-sonnet") || m.contains("claude-3.5-sonnet") {
        return Some(ModelPrice {
            input_per_1m: 3.00,
            output_per_1m: 15.00,
        });
    }
    if m.contains("claude-3-5-haiku") || m.contains("claude-3.5-haiku") {
        return Some(ModelPrice {
            input_per_1m: 0.80,
            output_per_1m: 4.00,
        });
    }
    if m.contains("claude-opus-4") || m.contains("claude-4-opus") {
        return Some(ModelPrice {
            input_per_1m: 15.00,
            output_per_1m: 75.00,
        });
    }
    if m.contains("claude-3-opus") {
        return Some(ModelPrice {
            input_per_1m: 15.00,
            output_per_1m: 75.00,
        });
    }
    if m.contains("claude-3-sonnet") {
        return Some(ModelPrice {
            input_per_1m: 3.00,
            output_per_1m: 15.00,
        });
    }
    if m.contains("claude-3-haiku") {
        return Some(ModelPrice {
            input_per_1m: 0.25,
            output_per_1m: 1.25,
        });
    }
    if m.starts_with("claude") {
        return Some(ModelPrice {
            input_per_1m: 3.00,
            output_per_1m: 15.00,
        });
    }
    if m.contains("gpt-4o-mini") {
        return Some(ModelPrice {
            input_per_1m: 0.15,
            output_per_1m: 0.60,
        });
    }
    if m.contains("gpt-4o") {
        return Some(ModelPrice {
            input_per_1m: 2.50,
            output_per_1m: 10.00,
        });
    }
    if m.contains("gpt-4-turbo") {
        return Some(ModelPrice {
            input_per_1m: 10.00,
            output_per_1m: 30.00,
        });
    }
    if m.contains("gpt-4") {
        return Some(ModelPrice {
            input_per_1m: 30.00,
            output_per_1m: 60.00,
        });
    }
    if m.contains("gpt-3.5") {
        return Some(ModelPrice {
            input_per_1m: 0.50,
            output_per_1m: 1.50,
        });
    }
    if m.contains("o4-mini") || m.contains("o3-mini") {
        return Some(ModelPrice {
            input_per_1m: 1.10,
            output_per_1m: 4.40,
        });
    }
    if m.contains("o3") {
        return Some(ModelPrice {
            input_per_1m: 10.00,
            output_per_1m: 40.00,
        });
    }
    if m.contains("o1-mini") {
        return Some(ModelPrice {
            input_per_1m: 1.10,
            output_per_1m: 4.40,
        });
    }
    if m.contains("o1") {
        return Some(ModelPrice {
            input_per_1m: 15.00,
            output_per_1m: 60.00,
        });
    }
    if m.contains("gemini-2.5-pro") {
        return Some(ModelPrice {
            input_per_1m: 1.25,
            output_per_1m: 5.00,
        });
    }
    if m.contains("gemini-2.5-flash") {
        return Some(ModelPrice {
            input_per_1m: 0.15,
            output_per_1m: 0.60,
        });
    }
    if m.contains("gemini-2.0-flash") {
        return Some(ModelPrice {
            input_per_1m: 0.10,
            output_per_1m: 0.40,
        });
    }
    if m.contains("gemini-1.5-flash") || m.contains("gemini-flash") {
        return Some(ModelPrice {
            input_per_1m: 0.075,
            output_per_1m: 0.30,
        });
    }
    if m.contains("gemini-1.5-pro") || m.contains("gemini-pro") {
        return Some(ModelPrice {
            input_per_1m: 1.25,
            output_per_1m: 5.00,
        });
    }
    None
}

pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> Option<f64> {
    let price = model_price(model)?;
    let cost = (input_tokens as f64 / 1_000_000.0) * price.input_per_1m
        + (output_tokens as f64 / 1_000_000.0) * price.output_per_1m;
    Some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_sonnet_pricing() {
        let p = model_price("claude-3-5-sonnet-20241022").unwrap();
        assert_eq!(p.input_per_1m, 3.00);
        assert_eq!(p.output_per_1m, 15.00);
    }

    #[test]
    fn test_gpt4o_mini_pricing() {
        let p = model_price("gpt-4o-mini").unwrap();
        assert_eq!(p.input_per_1m, 0.15);
        assert_eq!(p.output_per_1m, 0.60);
    }

    #[test]
    fn test_gemini_flash_pricing() {
        let p = model_price("gemini-1.5-flash").unwrap();
        assert_eq!(p.input_per_1m, 0.075);
    }

    #[test]
    fn test_estimate_cost_gpt4o() {
        let cost = estimate_cost("gpt-4o", 1000, 500).unwrap();
        let expected = (1000.0 / 1_000_000.0) * 2.50 + (500.0 / 1_000_000.0) * 10.00;
        assert!((cost - expected).abs() < 1e-9);
    }

    #[test]
    fn test_unknown_model_returns_none() {
        assert!(model_price("unknown-model-xyz").is_none());
    }

    #[test]
    fn test_claude_4_sonnet_pricing() {
        let p = model_price("claude-sonnet-4-20250514").unwrap();
        assert_eq!(p.input_per_1m, 3.00);
        assert_eq!(p.output_per_1m, 15.00);
    }

    #[test]
    fn test_claude_4_sonnet_alternate_pricing() {
        let p = model_price("claude-4-sonnet-20250514").unwrap();
        assert_eq!(p.input_per_1m, 3.00);
        assert_eq!(p.output_per_1m, 15.00);
    }

    #[test]
    fn test_o3_mini_pricing() {
        let p = model_price("o3-mini").unwrap();
        assert_eq!(p.input_per_1m, 1.10);
        assert_eq!(p.output_per_1m, 4.40);
    }

    #[test]
    fn test_o3_pricing() {
        let p = model_price("o3").unwrap();
        assert_eq!(p.input_per_1m, 10.00);
        assert_eq!(p.output_per_1m, 40.00);
    }

    #[test]
    fn test_o4_mini_pricing() {
        let p = model_price("o4-mini").unwrap();
        assert_eq!(p.input_per_1m, 1.10);
        assert_eq!(p.output_per_1m, 4.40);
    }

    #[test]
    fn test_gemini_25_flash_pricing() {
        let p = model_price("gemini-2.5-flash-preview-05-20").unwrap();
        assert_eq!(p.input_per_1m, 0.15);
        assert_eq!(p.output_per_1m, 0.60);
    }

    #[test]
    fn test_gemini_25_pro_pricing() {
        let p = model_price("gemini-2.5-pro-preview-05-20").unwrap();
        assert_eq!(p.input_per_1m, 1.25);
        assert_eq!(p.output_per_1m, 5.00);
    }
}
