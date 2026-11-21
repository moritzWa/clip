use crate::store::Preview;
use std::collections::VecDeque;

/// Adjacent text clips become one paste, separated by a newline. Images/files
/// stay separate so the receiving application gets their native data types.
pub fn paste_plan(previews: Vec<Preview>) -> VecDeque<Preview> {
    let mut plan: VecDeque<Preview> = VecDeque::new();
    for preview in previews {
        let text_only = preview.image_path.is_none() && preview.file_path.is_none();
        if text_only {
            if let Some(last) = plan
                .back_mut()
                .filter(|p| p.image_path.is_none() && p.file_path.is_none())
            {
                last.text.push('\n');
                last.text.push_str(&preview.text);
                continue;
            }
        }
        plan.push_back(preview);
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    fn text(value: &str) -> Preview {
        Preview {
            text: value.into(),
            image_path: None,
            file_path: None,
            source: String::new(),
            created_at: String::new(),
            kind: "text".into(),
            bytes: None,
        }
    }
    #[test]
    fn combines_text_without_trimming_or_reordering() {
        let plan = paste_plan(vec![text("  first\n"), text("second"), text("third\n\n")]);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].text, "  first\n\nsecond\nthird\n\n");
        assert_eq!(
            paste_plan(vec![text("  single\n\n")])[0].text,
            "  single\n\n"
        );
    }
    #[test]
    fn mixed_clips_keep_their_order_and_native_types() {
        let mut image = text("OCR is not a text paste");
        image.image_path = Some("synthetic.png".into());
        let mut file = text("");
        file.file_path = Some("synthetic.txt".into());
        let plan = paste_plan(vec![text("before"), image, text("after"), file]);
        assert_eq!(plan.len(), 4);
        assert_eq!(plan[0].text, "before");
        assert!(plan[1].image_path.is_some());
        assert_eq!(plan[2].text, "after");
        assert!(plan[3].file_path.is_some());
        assert!(paste_plan(Vec::new()).is_empty());
    }
}
