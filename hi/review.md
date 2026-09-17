---
hi: 1
families: [REVIEW]
---

# Reviewing changes

## Intent

Before a change reaches a human reviewer, a model should already have read the diff with the project's own contracts in hand. The review should stay inside the diff, so it reads like a colleague's comments rather than a proposal to rewrite the repository. For a change that matters, several models should be able to read exactly the same diff at once — where they disagree is the part worth a second look.

## Criteria

- **REVIEW-1**  I can have my changes reviewed before another person has to look at them.
  - **REVIEW-1.a**  The review covers the diff against the branch I will merge into.
  - **REVIEW-1.b**  fledge works out what that base branch is without being told.
  - **REVIEW-1.c**  I can point the review at a single file when that is the part I am least sure about.
  - **REVIEW-1.d**  A diff with nothing in it says so rather than asking a model about nothing.
  - **REVIEW-1.e**  I see what the diff contains before I read the review of it.
- **REVIEW-2**  The review is told about the module contracts the diff touches.
  - **REVIEW-2.a**  I can add contracts the review would not have picked up on its own.
  - **REVIEW-2.b**  A missing or broken contract never blocks a review.
- **REVIEW-3**  The review stays inside the diff instead of proposing rewrites of code I did not touch.
- **REVIEW-4**  I can steer what the review looks for.
  - **REVIEW-4.a**  I can decide how the findings are laid out.
- **REVIEW-5**  I can get a second and third opinion from other models on exactly the same diff.
  - **REVIEW-5.a**  The models run at the same time rather than one after another.
  - **REVIEW-5.b**  Every model sees identical input, so disagreement between them means something.
  - **REVIEW-5.c**  One model failing still leaves me the reviews from the others.
  - **REVIEW-5.d**  The opinions come back in the order I asked for them, not the order they finished.
