---
name: grill-me
description: >-
  Engage in an interactive interview (grilling) with the user to clarify requirements,
  resolve architectural decisions, map out edge cases, and evaluate design trade-offs
  before writing code.
---

# Grill-Me (Interactive Interview) Skill

This skill defines the protocol for "grilling" the user—conducting an interactive, deep-dive interview to align on design decisions and implementation plans.

## Core Principles

1. **Be Inquisitive & Thorough**: Don't accept vague requirements. Ask about edge cases, error handling, performance constraints, and UI/UX subtleties.
2. **One Step at a Time**: Do not overwhelm the user with a massive list of questions all at once. Ask 2-3 highly targeted questions at a time, wait for answers, and then ask follow-ups.
3. **Present Options & Trade-offs**: For every major design choice, offer 2-3 concrete options with their pros and cons.
4. **Active Listening**: Restate the user's answers in your own words to confirm alignment.

## Grilling Process

### Phase 1: Scope & Goal Discovery
Understand what the user wants to achieve and why:
- What is the business goal or problem being solved?
- Are there any existing constraints (e.g., matching a specific API, using a specific library)?

### Phase 2: Technical Deep-Dive
Probe the technical details:
- **Data flow**: How does data move through the system?
- **State management**: Where does the state live?
- **Error states**: What happens when an API call fails or inputs are invalid?
- **Edge cases**: What are the extreme inputs or rare user behaviors?

### Phase 3: Consensus & Plan
Summarize the decisions made during the interview and draft a final implementation plan.
