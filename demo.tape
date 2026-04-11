# tuillem demo recording
# Run with: vhs demo.tape
#
# Prerequisites:
# - tuillem installed and configured with a working provider
# - vhs installed (https://github.com/charmbracelet/vhs)

Output demo.gif
Output demo.mp4

Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 800
Set Theme "Catppuccin Mocha"
Set Padding 20
Set Framerate 30
Set TypingSpeed 50ms

# Start tuillem
Type "tuillem"
Enter
Sleep 2s

# Show the interface
Sleep 1s

# Type a message
Ctrl+N
Sleep 1s
Type "Hello! Can you give me a brief overview of the Rust programming language in a few bullet points?"
Sleep 500ms
Enter
Sleep 8s

# Let the response stream and settle
Sleep 3s

# Scroll through the response with Enter
Enter
Sleep 2s
Enter
Sleep 2s

# Show model switching with Ctrl+O
Ctrl+O
Sleep 1s
# Navigate the model list
Down
Sleep 500ms
Down
Sleep 500ms
Escape
Sleep 1s

# Show the command palette with Ctrl+K
Ctrl+K
Sleep 1.5s
Escape
Sleep 1s

# Show slash command: /new with prompt
Type "/new Write a 'Survival Guide for New Rustaceans' written by a weary, caffeine-addicted senior developer. Use heavy markdown formatting (bolding, italics, blockquotes) to convey dramatic flair. Include a table titled 'The Hierarchy of Rust Frustration' that ranks common errors (like Borrow Checker battles or lifetime annotations) by 'Mental Stability Lost' and 'Recommended Coping Mechanism.' End with a slightly sarcastic motivational quote."
Sleep 500ms
Enter
Sleep 10s
Enter
Sleep 2s
Enter
Sleep 1s
Enter
Sleep 1s
Enter
Sleep 1s
Enter
Sleep 1s
Enter

# Let response stream
Sleep 1s
Enter

# Show sidebar navigation
Tab
Sleep 500ms
Down
Sleep 500ms
Down
Sleep 500ms
Up
Sleep 500ms
Enter
Sleep 1s

# Show search
Tab
Sleep 500ms
Type "/"
Sleep 300ms
Type "rust"
Enter
Sleep 1s
Down
Sleep 1s
Down
Sleep 1s
Down
Sleep 1s
Escape
Sleep 1s

# Show help overlay
Ctrl+H
Sleep 3s
# Scroll help
Down
Down
Down
Sleep 1s
Escape
Sleep 1s

# Show settings
Ctrl+S
Sleep 2s
# Navigate settings
Down
Sleep 300ms
Down
Sleep 300ms
Down
Sleep 300ms
Escape
Sleep 1s

# Toggle sidebar
Ctrl+L
Sleep 1.5s
Ctrl+L
Sleep 1.5s

# Copy response
Ctrl+Y
Sleep 1.5s

# Show stats
Type "/stats"
Enter
Sleep 2s

# Final pause to show the app
Sleep 3s

# Quit
Ctrl+C
Sleep 1s
