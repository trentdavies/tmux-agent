# Phase 3: Attention System

## What
Normalized event streaming system with cursor-based replay for operator agents.

## Why
Operator agents need a structured feed of events to make decisions about when to intervene, restart, or send new prompts to coding agents.

## Key Features
- Event categories: Session, Pane, Agent, Alert, System, Health
- Actionability levels: Background, Interesting, ActionRequired
- Bounded ring buffer journal (10k events, 1hr retention)
- Cursor-based replay with expiration detection
- Stateless poll and long-running watch modes
