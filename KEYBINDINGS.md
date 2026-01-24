# Key Bindings

| View                   | Key                                       | Action                                                 |
| ---------------------- | ----------------------------------------- | ------------------------------------------------------ |
| **Global**             | `?`                                       | Show help modal for current view                       |
| **Main**               | `n`                                       | Create new review                                      |
| **Main**               | `Up` / `Down` / `k` / `j`                 | Change review selection                                |
| **Main**               | `o` / `Space` / `Enter`                   | Open selected review                                   |
| **Main**               | `d`                                       | Delete selected review                                 |
| **Main**               | `r`                                       | Open refresh review chooser                            |
| **Main**               | `q` / `Ctrl+C`                            | Quit application                                       |
| **Review create**      | `Up` / `Down` / `k` / `j`                 | Change branch selection                                |
| **Review create**      | `Tab`                                     | Switch between target and base branch selection        |
| **Review create**      | `Enter`                                   | Submit review                                          |
| **Review create**      | `Esc`                                     | Cancel and close popup                                 |
| **Review details**     | `Up` / `Down` / `k` / `j`                 | Change file or line selection                          |
| **Review details**     | `Enter`                                   | Switch between files lists and content box             |
| **Review details**     | `Space`                                   | When in files list, toggle file viewed                 |
| **Review details**     | `c`                                       | Open comments view for currently selected file or line |
| **Review details**     | `r`                                       | Open refresh review chooser                            |
| **Review details**     | `Esc`                                     | Close review details / go back to main view            |
| **Refresh review**     | `b`                                       | Refresh base SHA                                       |
| **Refresh review**     | `t`                                       | Refresh target SHA                                     |
| **Refresh review**     | `a`                                       | Refresh both SHAs                                      |
| **Refresh review**     | `d`                                       | Duplicate review from current heads                    |
| **Refresh review**     | `Up` / `Down` / `k` / `j`                 | Move selection                                         |
| **Refresh review**     | `Enter`                                   | Select action                                          |
| **Refresh review**     | `Esc`                                     | Cancel                                                 |
| **Comments**           | `Enter`                                   | Submit comment                                         |
| **Comments**           | `Tab`                                     | Switch focus between input and comments list           |
| **Comments**           | `Up` / `Down` / `k` / `j` (comments list) | Change comment selection                               |
| **Comments**           | `r` (comments list)                       | Mark currently selected comment as resolved            |
| **Comments**           | `R` (comments list)                       | Mark all comments as resolved                          |
| **Comments**           | `Esc`                                     | Close comments                                         |
| **ConfirmationDialog** | `y` / `Y` / `Enter`                       | Confirm                                                |
| **ConfirmationDialog** | `n` / `N` / `Esc`                         | Cancel                                                 |
| **Help Modal**         | `Up` / `Down` / `k` / `j`                 | Navigate keybindings                                   |
| **Help Modal**         | `Enter`                                   | Execute selected action                                |
| **Help Modal**         | `Esc`                                     | Close help modal                                       |

> NOTE: The refresh review dialog disables unavailable actions (shown as `N/A`); if no SHAs can be refreshed, only `Esc` is available.
