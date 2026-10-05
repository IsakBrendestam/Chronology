# Game Design

The following design decisions has been made:
* A communication layer was implemented to enable communication between the client backend and the ui (frontend). The reason for this design was to enable communication where the backend can run asynchronously while the ui is running synchronously. This essentially means that the backend interprets any message from the server and sends corresponding action to the frontend, the frontend takes actions according to the received information and then communicates back to the backend. The backend finally reports back to the server.

* Another important design decision is that the state of the ui is only manipulated within the ```receive_client_messages```. This means that the only way the state can be changed is if the backend sends a message through the communication layer to the ui. The reason for this is mainly to make the game easier to debug, if there is something with the game state the cause is most likely backend related.

* Due to the strong ECS design of Bevy, internal messages within the frontend were used frequently with the ```MessageWriter``` and ```MessageReader``` data structures. This does make the code harder to follow, since there is no natural flow in the code. In some cases this can probably be prevented. However using this approach did speed up development for such a small project.

## Security Overview  

### Server
The server interprets the client as untrusted, and therefore has to consider multiple risks:

| Threat | Risk | Mitigation | 
| - | - | - |
| Manipulated messages | Game logic exploitation | Continuous validation |
| Message Injection | Unauthorized players | Cookie tokens |
| Identity theft by forged cookies | Unauthorized players | cookies stored locally and never shared.
| Denial of Service (DoS) | Server CPU overload | Continuous message count limit |
| Desynchronized messages | Game logic corruption | State machine validation |
| Information Disclosure | Game cheating | Server only sends required information |


### Client
Even though the client is untrusted from the server perspective it still has local risks:
| Threat | Risk | Mitigation | 
| - | - | - |
| Resource Exhaustion (e.g. too large meshes) | Exhausted memory | Limit asset sizes |
| Oversized message | Exhausted memory | Message size limit |


<details>
<summary>
Grade4 Design
</summary>

* The client is mainly responsible for contacting the server. This simplifies the server somewhat due to not having to contain logic related to the mechanics of the game or the specific state of each player. However the server still has to keep track of the player turn and thus also the logic related to changing player turns in round robin style. Even though the server does not currently support multiple “game sessions” running simultaneously, implementing this should be fairly straightforwards.

* The first player to join the server will be assigned “Session owner”. It’s this player that will be responsible for starting the game once all players have joined. The reason for this decision is 
to avoid having to manually interact with the server. This also enables easier implementation of multiple game sessions running simultaneously in the future. 

* Both the client and server are running asynchronously, this makes for better CPU utilization due to non-blocking I/O operations. It also allows for lower memory usage, due to less stack allocations.

* If a player is disconnected, the game will continue with the next player. This makes sure that the game won't stall due to one (or multiple) player(s) losing connection.
</details>