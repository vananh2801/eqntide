# Spec Delta

## ADDED Requirements

### Requirement: About external links dispatch to an active system browser handler
The **About** tab SHALL wire its external browser actions—including the **GitHub** repository hyperlink, **Open release** / **Download**, **Release notes**, and **Open Releases**—to an enabled platform URL handler so clicking any validated link invokes the user's system browser across all desktop renderer backends.

#### Scenario: Clicking the About GitHub repository link opens the repository page
- **WHEN** the user clicks the **GitHub** repository hyperlink in the **About** tab
- **THEN** the desktop application dispatches the repository URL to the enabled platform URL handler so the system browser opens the repository page

#### Scenario: Clicking an About release or error action opens the validated URL in the browser
- **WHEN** the user clicks **Open release**, **Download**, **Release notes**, or **Open Releases** in the **About** tab with a trusted GitHub URL
- **THEN** the desktop application both emits the URL command and routes it through an active system browser integration rather than dropping it as a disabled-feature no-op
