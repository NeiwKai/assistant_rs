## Install with cargo </br>
`cargo install --git https://github.com/NeiwKai/assistant_rs` </br></br>

*Please make sure that you include `chat_history.json` in the same directory that install the program* </br>

Your file structure should be some thing like </br>
```
.cargo
└── bin
    ├── assistant_rs
    └── chat_history.json
```

### Example of `chat_history.json` </br>
```
{
  "messages": [
    {
      "content": "YOUR PROMPT"
      "role": "system"
    }
  ]
}
```
