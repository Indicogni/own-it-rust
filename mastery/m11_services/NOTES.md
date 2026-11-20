# Mastery M11 notes

[Module guide](README.md) · [Concepts](CONCEPTS.md) · [Hints](HINTS.md)

Your workbook. Write answers in your own words, *before* you open the hints. Come back after one day, three days, a week and three weeks: answer the mastery questions at the bottom again from memory, then compare with what you wrote here.

## l01 · Service anatomy

**Predictions I got wrong, and why:**

- 

**Explain:** Follow one `POST /parts` request from the socket to the response. Name each step (routing, extractors, handler, `IntoResponse`) and say where the shared state is cloned and where it is locked.

> 

**Edge case I added:**

- 

## l02 · Middleware with tower

**Predictions I got wrong, and why:**

- 

**Explain:** Draw your `hardened` stack as nested boxes. For a request that times out, list which layers still see the response, and say why the response still carries a request id. Then explain why `InFlightLimit` needs a value with a `Drop` impl and cannot just subtract after `.await`.

> 

**Edge case I added:**

- 

## l03 · Persistence

**Predictions I got wrong, and why:**

- 

**Explain:** Why does `move_units` need a transaction even though each `UPDATE` is atomic on its own? What happens to a request that needs a connection when the pool is empty, and what should the client see? Why is the listing with one query per shelf a problem only at scale?

> 

**Edge case I added:**

- 

## l04 · Authentication

**Predictions I got wrong, and why:**

- 

**Explain:** Why must a password hash be slow and salted while the token MAC must be fast? Why does `verify_token` check the MAC before it parses the payload? What exactly does the decoy hash protect, and what does it not protect?

> 

**Edge case I added:**

- 

## l05 · Validation and API design

**Predictions I got wrong, and why:**

- 

**Explain:** Where exactly is the boundary in your code, that is, the last line that can see an unvalidated value? Why does keyset pagination survive a delete that offset pagination does not? Why must a reused idempotency key with a different body be an error instead of a replay?

> 

**Edge case I added:**

- 

## l06 · CPU-bound work in a service

**Predictions I got wrong, and why:**

- 

**Explain:** Why does a blocked async worker thread hurt requests that have nothing to do with the slow one? Why does `CpuLimiter` release its slot when the job ends and not when the caller goes away? When is `spawn_blocking` enough and when do you want a rayon pool?

> 

**Edge case I added:**

- 

## l07 part 1 · A binary protocol: the shared crate

**Predictions I got wrong, and why:**

- 

**Explain:** Which check in `decode_frame` protects memory, which detects damage, and which allows the protocol to change later? Why must the length be checked before the payload has arrived? What does the checksum not protect against?

> 

**Edge case I added:**

- 

## l07 part 2 · A binary protocol: the connection

**Predictions I got wrong, and why:**

- 

**Explain:** What is backpressure, and which line of your `receive` creates it? What could a sender conclude from an `Ack` if the receiver sent it before handing the batch on? Why is an unbounded queue a bug that only shows under load?

> 

**Edge case I added:**

- 

## l08 · Observability and operations

**Predictions I got wrong, and why:**

- 

**Explain:** What question does `/healthz` answer and what question does `/readyz` answer, and who asks each? Why must the `route` label come from the route template and not from the request path? Why does a span entered with `enter()` and held across `.await` produce wrong logs?

> 

**Edge case I added:**

- 

## l09 · Testing services

**Predictions I got wrong, and why:**

- 

**Explain:** Which of your tests need a socket, and why do the others not? Why does every test get its own database? How did you make the database fail on purpose, and what did that test tell you that no "happy path" test could?

> 

**Edge case I added:**

- 

## l08 lab · The service in a container

**What I ran and what I saw, per checklist line:**

- 

**Answers to the three lab questions:**

> 

## c01 · Code review: the field-notes service

**My review, written before running the tests** (for each defect: what an attacker or a busy day would do with it):

1. 
2. 
3. 
4. 
5. 
6. 

**What the tests found that I missed:**

- 

## Capstone · beacon

**Design, written before coding.** What does an `Ack` promise? What makes two samples the same? Where is every bound, and what happens at each? Which tasks exist and what do they share? How does each program stop?

> 

**After it passes.** Under which failure does my agent lose a sample, and how would an operator notice? What would change for two servers behind a load balancer?

> 

**Self-review checklist from the brief: what I had to fix:**

- 

## Mastery check

Answer from memory. Model answers are at the end of [HINTS.md](HINTS.md#mastery-check--model-answers).

1. A request arrives at an axum router. Name the steps until a response is written, and say which steps can answer with an error before your handler runs. Why must the body extractor be the last parameter?

   > 

2. You write `.layer(a).layer(b)` on a router, and `ServiceBuilder::new().layer(a).layer(b)` elsewhere. Which layer is outermost in each? Where does a timeout layer go if timed-out responses must carry a request id?

   > 

3. A request future is dropped at an `.await`. Which of your code still runs? What follows for cleanup such as releasing a slot or decrementing a gauge?

   > 

4. A function begins a transaction, runs one `UPDATE`, and returns early with `?` from a second statement. What is in the database, and why?

   > 

5. All connections of a pool are in use. What happens to the next query, what should the client see, and which design mistakes produce this situation without real load?

   > 

6. Why must a password hash be slow, salted and self-describing, while a token MAC may be fast?

   > 

7. What makes a login endpoint leak which user names exist, and what do you do about each leak? What does a decoy hash not protect against?

   > 

8. Compare offset and keyset pagination when a row is deleted between two page requests.

   > 

9. How does an idempotency key make a retried `POST` safe? What must be atomic, and what happens when the key is reused with another body?

   > 

10. A handler computes for 200 ms without an `.await`. Describe the effect on a runtime with four worker threads under load, and two ways to fix it.

   > 

11. You receive bytes from a TCP connection. In what order do you check magic, version, length, checksum and payload contents, and what are the three possible answers of a frame decoder?

   > 

12. What is backpressure? Describe a receiver that has it and one that does not, and what happens to each when its consumer stalls.

   > 

13. Define liveness and readiness. Which of them may depend on the database? What is the order of events in a graceful shutdown?

   > 

14. Which behaviour of a service can a handler test without a socket verify, and for which do you need a socket or a child process?

   > 

15. Why does tokio's paused test clock give wrong results when real threads or real I/O take part?

   > 
