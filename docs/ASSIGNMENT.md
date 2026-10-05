# D7082E_GRADE5_GUI

For Grade 5 one option is to look at your Grade 4 client side implementation, giving it a GUI (Graphical User Interface).

If you have collaborated with others. Change the below to match the way you collaborated. In this way, suspicion of plagiarism can be avoided even if your repos turn out to be very similar.

- [](), Mr X, came up with the idea of Y which I used in my code.
- [](), Mrs Z, suggested using R, which turned ut great, so I used that as well.
- Etc.

## Design Choices

Document and motivate design choices made throughout the lab in [GAME_DESIGN.md](GAME_DESIGN.md).

You may choose to use your Grade 4 server as is (just implement the client GUI in this repo), or include the server in this repo. For the latter you may either use workspaces or make it multi binary crate. To include the server may be favorable if you want to make further changes to the server side.

## Task 1 - Client GUI

The client GUI should replicate and optionally extend on the functionality provided in your CLI Grade 4 version. You are free to select any GUI framework, make sure to motivate your technical design choices in [GAME_DESIGN.md](GAME_DESIGN.md), and provide end-user instructions in the [GAME.md](GAME.md).

There are many GUI frameworks to choose from, among them you find [ratatui](https://crates.io/crates/ratatui/), [egui](https://crates.io/crates/egui) and [iced](https://crates.io/crates/egui). For the latter you can look at [d7082e_iced](https://vesuvio-git.neteq.ltu.se/d7082e/d7082e_iced) for a rough sketch to get started. Alternatively you may opt for using the [Bevy](https://crates.io/crates/bevy) game engine. It gives you more control over the rendering.

Testing GUI widgets and whole GUI applications is in general challenging. Try following the best practices by breaking down your GUI into smaller components and test them separately, e.g., by creating your own library, and use the `examples`folder to showcase their use. Once your GUI components (widgets) are in place integrate them into the client side game application.

## Task 3 - Audio/Sound

Interactive applications benefit from audio feedback. Dependent on your choice of GUI framework, audio might be integrated. If not the [CPAL](https://crates.io/crates/cpal) provides a simple cross-platform framework.

Audio/sound testing is similarly to GUI challenging. Apply best practices by developing a library of building blocks and test/evaluate them in separation before integrating into your game client application.

## Task 4 - Robustness, reliability and Security

- Avoid `unwrap` and other operations that may lead to `panic!`. Make clear comments in your code for remaining cases where errors are unrecoverable, and motivate why the error is unrecoverable. This relates to robustness and reliability of your game client.

- Make a security overview of both the client and server side application, define threat model, risk, possible mitigations etc.

Implementing mitigations are optional, but since this is a course given within the Cyber Secures Master's program, we should take security into account.

## Optional

You may choose the extend the game in various ways:

- Multiple game modes, e.g., a "sports" mode with categories related to sports only, e.g., "tennis, football, golf, F1, etc.", another mode "music", with categories such as "rock", "pop", "classical", "metal", etc. The lobby could present a set of pre-defined modes or let the players crate a new mode from the set of categories in the database. This could be combined with multi-game option discussed above, but should also work in a single game scenario.

- Whatever you feel that the ultimate Chronology championship craves!

- Update [GAME_DESIGN.md](GAME_DESIGN.md), [GAME.md](GAME.md) and/or other files as needed. Extending the game, will of course also affect the server side.

## Learning Outcomes

- Reflect in your own words what you have learned by doing this lab. Hint, compare your knowledge regarding Rust, tooling etc., before and after doing this lab.

This assignment gave me an extremely fun introduction to the bevy game engine. I learned a lot about the use of rust in game development, coming from C++ it’s a relief not having to worry about deleted memory and ownership. It was also interesting to experience the full commitment to ECS in Bevy, it showed where ECS shines but also where it falls short. I ended up using bevy’s message system a lot which probably isn’t the most performant solution.

## Congrats

You have now taken a step into GUI programming in Rust, where you reaped the benefits of ready-made libraries and the Rust ecosystem. The upside is as usual, performance, robustness, reliability, and the language level support for building safety and secure software.
