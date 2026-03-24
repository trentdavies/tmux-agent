# Phase 4: Alert Management

## What
Alert system for detecting and managing agent panes that need human attention.

## Why
When running multiple agents, you need to know which ones are stuck, crashed, rate-limited, or running low on context — without manually checking each pane.

## Key Features
- Alert types: AgentStuck, AgentCrashed, AgentError, RateLimit, ContextWarning
- Alert lifecycle: active → acked/muted/resolved
- Deduplication by (session, pane, type)
- Health checks: output velocity tracking, stall detection, error pattern matching
- Health states: Healthy, Degraded, Unhealthy, RateLimited
