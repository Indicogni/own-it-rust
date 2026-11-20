# Chapter 18 notes — Files, streams and processes

Your workbook. Write answers in your own words, *before* checking the hints. `cargo progress` tells you when this chapter is due for a review and which part of these notes that review uses.

## l01 · Streaming bytes

**Predictions I got wrong, and why:**

- 

**Explain:** Why use Read rather than File in the contract? Explain EOF versus Interrupted.

> 

**Edge case I added:**

- 

## l02 · Buffered text

**Predictions I got wrong, and why:**

- 

**Explain:** Why can byte records succeed where text lines return InvalidData?

> 

**Edge case I added:**

- 

## l03 · Writers and flushing

**Predictions I got wrong, and why:**

- 

**Explain:** Why should a function explicitly propagate flush failure?

> 

**Edge case I added:**

- 

## l04 · Seeking and binary formats

**Predictions I got wrong, and why:**

- 

**Explain:** Explain why field sizes and byte order belong to a format contract.

> 

**Edge case I added:**

- 

## l05 · Paths preserve OS data

**Predictions I got wrong, and why:**

- 

**Explain:** Distinguish lexical path processing from filesystem canonicalization and security containment.

> 

**Edge case I added:**

- 

## l06 · Directory metadata

**Predictions I got wrong, and why:**

- 

**Explain:** Why should a listing specify symlink and ordering policies?

> 

**Edge case I added:**

- 

## l08 · Framed messages over TCP

**Predictions I got wrong, and why:**

- 

**Explain:** Why does a TCP protocol need framing? Why is the declared length checked before the body buffer is created? Why does the client shut down only its writing half instead of closing the connection?

> 

**Edge case I added:**

- 

## l09 · Child processes and pipes

**Predictions I got wrong, and why:**

- 

**Explain:** Why is a failed child not an io::Error? Who must close which end of a pipe before a reader sees the end of the stream, and what happens when a pipe is full?

> 

**Edge case I added:**

- 

## l07 · Capstone: a bounded record service

**Predictions I got wrong, and why:**

- 

**Explain:** Explain framing, length validation before allocation, error propagation, and when the response is flushed.

> 

**Edge case I added:**

- 

## r01 · Spaced retrieval: earlier chapters inside Chapter 18

**Predictions I got wrong, and why:**

- 

**Edge case I added:**

- 

## c01 · A teammate's pull request

**My review, written before running the tests:**

- 

**Edge case I added:**

- 

## Drills

Drills whose verdict surprised me, and the rule I learned:

- 

## Mastery check (answer from memory)

**What may a successful Read::read or Write::write return before the buffer is exhausted?**

> 

**When does read_line append, and how do lines() and split() differ?**

> 

**How do Cursor, BufReader, Take, Chain, and Seek compose?**

> 

**Why must paths and file names not be forced through String?**

> 

**Which error kinds should retry, propagate, or end a stream?**

> 

**How do you frame messages on TCP, and why does a single read not correspond to a whole message?**

> 

**Why is a failed child process not an io::Error, and how can a parent and a child connected by pipes block each other forever?**

>
