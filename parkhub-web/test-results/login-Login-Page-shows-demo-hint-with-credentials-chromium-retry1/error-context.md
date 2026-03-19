# Page snapshot

```yaml
- generic [ref=e3]:
  - generic [ref=e6]:
    - generic [ref=e7]:
      - img [ref=e9]
      - generic [ref=e11]: ParkHub
    - heading "Your parking, your server, your rules." [level=2] [ref=e12]:
      - text: Your parking,
      - text: your server,
      - text: your rules.
    - paragraph [ref=e13]: Self-hosted parking management. No cloud, no tracking, no monthly fees. Runs on your infrastructure.
  - generic [ref=e15]:
    - heading "Sign In" [level=1] [ref=e16]
    - paragraph [ref=e17]: Welcome back to ParkHub
    - generic [ref=e18]: "Demo: admin@parkhub.test / demo"
    - generic [ref=e20]:
      - generic [ref=e21]:
        - generic [ref=e22]: Email
        - textbox "Email" [ref=e23]:
          - /placeholder: admin@parkhub.test
      - generic [ref=e24]:
        - generic [ref=e25]:
          - generic [ref=e26]: Password
          - link "Forgot password?" [ref=e27] [cursor=pointer]:
            - /url: /forgot-password
        - generic [ref=e28]:
          - textbox "Password" [ref=e29]:
            - /placeholder: demo
          - button "Show password" [ref=e30]:
            - img [ref=e31]
      - button "Sign In" [disabled] [ref=e33]
    - paragraph [ref=e34]:
      - text: Don't have an account?
      - link "Sign Up" [ref=e35] [cursor=pointer]:
        - /url: /register
    - paragraph [ref=e36]: ParkHub v1.4.6
```