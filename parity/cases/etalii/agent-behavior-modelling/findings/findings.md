# Findings

Every rule of the tool, once.

## Behavior

- **Do in order:** First root
  - Read the request
  - **Check:** The request is clear
    - **Do:** A child a check cannot hold
  - **Retry up to 0 times:** Never
    - **Do:** Try it
  - **Only while:** Nothing beneath
  - **Ask approval before:** Two beneath
    - **Do:** One
    - **Do:** Two
  - **Try in order:** Empty alternatives
- **Do:** Second root
