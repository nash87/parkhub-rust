# Page snapshot

```yaml
- generic [ref=e5]:
  - link "Back" [ref=e6] [cursor=pointer]:
    - /url: /welcome
    - img [ref=e7]
    - text: Back
  - generic [ref=e9]:
    - img [ref=e11]
    - generic [ref=e13]: ParkHub
  - heading "Sign In" [level=1] [ref=e14]
  - paragraph [ref=e15]: Welcome back to ParkHub
  - generic [ref=e16]: "Demo: admin@parkhub.test / demo"
  - generic [ref=e18]:
    - generic [ref=e19]:
      - generic [ref=e20]: Email
      - textbox "Email" [ref=e21]:
        - /placeholder: admin@parkhub.test
    - generic [ref=e22]:
      - generic [ref=e23]:
        - generic [ref=e24]: Password
        - link "Forgot password?" [ref=e25] [cursor=pointer]:
          - /url: /forgot-password
      - generic [ref=e26]:
        - textbox "Password" [ref=e27]:
          - /placeholder: demo
        - button "Show password" [ref=e28]:
          - img [ref=e29]
    - button "Sign In" [disabled] [ref=e31]
  - paragraph [ref=e32]:
    - text: Don't have an account?
    - link "Sign Up" [ref=e33] [cursor=pointer]:
      - /url: /register
  - paragraph [ref=e34]: ParkHub v1.4.6
```