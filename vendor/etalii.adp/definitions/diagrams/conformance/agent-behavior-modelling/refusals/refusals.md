# Refusals

## Behavior

- **Do in order:** Root
  - **Do:** A leaf
  - **Retry up to 3 times:** Wrapper
    - **Do:** Its child
  - **Try in order:** Alternatives
    - **Check:** One
