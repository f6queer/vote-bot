# F^6 telegram bot

## How it works?
If user votes the poll, poll token is generated by public key (poll key, 128 bits) and random private key (128 bits).
Then, it sends a private key to the user.

If user wants to cancel the vote, the user sends a private key and the voting is cancelled.

## TODO
- [x] Complete security poll
- [x] Token services
- [ ] Timer services