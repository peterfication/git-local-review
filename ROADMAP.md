# Roadmap

## Review base/target SHA change

### Show delta since review

Provide a secondary diff that compares the original target SHA to the current target SHA so users can see what changed without replacing the original review diff.

### Detect rebase and warn

If the current branch SHA is not a descendant of the original, surface a stronger warning that suggests creating a new review rather than refreshing in place.

### Ignore change

Allow users to dismiss the warning and keep the review bound to the original SHAs without further prompts.

### Edit SHAs manually

Offer a manual retarget action for cases where branch names exist but the review should be re-anchored to a different specific SHA.
