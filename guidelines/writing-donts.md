# Writing don'ts

An editorial checklist paraphrased from Charlie Guo's [The Field Guide to AI Slop](https://www.ignorance.ai/p/the-field-guide-to-ai-slop), published October 22, 2025. This is a summary, not a reproduction.

## What to avoid

- Don't overuse em dashes or other dramatic punctuation.
- Don't manufacture depth through repeated contrasts, three-part slogans, rhetorical questions, or sudden declarations of significance.
- Don't open with generic scene-setting or use transitions that add no information.
- Don't scatter bold text, decorative Unicode, or emojis without a clear purpose.
- Don't turn every explanation into a list when connected prose would communicate better.
- Don't repeat the same sentence rhythm and paragraph structure throughout a piece.
- Don't use plausible-sounding metaphors whose comparisons fail under scrutiny.
- Don't pad a point with sentences that merely restate it.
- Don't substitute polished generalities for specific knowledge, concrete detail, and a considered viewpoint.

## Apply with judgment

These are revision prompts, not blanket bans or an AI-authorship detector. Vocabulary, correct grammar, and contraction choices are not proof of AI use. Human writers use these devices too; assess their purpose and cumulative effect.

## Repository use

This checklist is loaded as a rule candidate, not executed. The companion
[writing-donts.yaml](writing-donts.yaml) activates three semantic checks: paragraph
filler, section repetition, and sentence metaphors. These are repository-authored
review instructions, not rules supplied by the article's author.

Run with `--guidelines ./guidelines` and a configured semantic-text provider and
model. Without a provider, these checks are reported as skipped and the review
is incomplete. Scope filtering still applies. Findings are editorial suggestions,
not authorship judgments. Oversized section checks retain the pipeline's partial
coverage behavior.
