# Roadmap

## Review base/target SHA change

### Keep review pinned, add refresh actions

Keep the review anchored to the original base/target SHAs and expose explicit actions for `Refresh target SHA`, `Refresh base SHA`, and `Refresh both`. On success, clear the `*_sha_changed` markers so the list reflects the refreshed state.

### Create new review from current heads

Preserve the old diff/comments by creating a new review using the current base/target branch heads, leaving the original review untouched for historical context.

### Show delta since review

Provide a secondary diff that compares the original target SHA to the current target SHA so users can see what changed without replacing the original review diff.

### Detect rebase and warn

If the current branch SHA is not a descendant of the original, surface a stronger warning that suggests creating a new review rather than refreshing in place.

### Ignore change

Allow users to dismiss the warning and keep the review bound to the original SHAs without further prompts.

### Edit SHAs manually

Offer a manual retarget action for cases where branch names exist but the review should be re-anchored to a different specific SHA.
