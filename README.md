## Install with cargo 
`cargo install --git https://github.com/NeiwKai/assistant_rs` </br></br>

*Please make sure that you run the command in the same directory as `chat_history.json`* </br>
*Then get a .gguf model file from <a href="https://huggingface.co/">hugging face</a>*

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
## Future Roadmap 
* web browsing ability
* file upload like .pdf, .txt
* text-to-speech
* chatbot avatar
