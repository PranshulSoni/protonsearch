# MemoryOS Killer App Strategy

## One-Line Vision

MemoryOS is the memory layer for Windows.

It helps users find, understand, and continue anything they have seen, opened, copied, written, searched, or worked on.

## Core Promise

The app should make users feel:

> My computer finally remembers everything for me.

The product should not be positioned as:

- A Raycast clone
- A launcher
- An AI assistant
- A Windows Search replacement
- A collection of power-user commands

The launcher is only the entry point. The real product is persistent, searchable, explainable computer memory.

## Brutal Product Diagnosis

The current app already has strong power-user features:

- Universal launcher
- File and document search
- Browser history and bookmark search
- Clipboard history
- Git search
- Window management
- System actions
- AI chat and commands
- Timeline tracking
- Focus tools
- Snippets and quicklinks

But the killer app is not "more commands".

The killer app is:

> Find what I forgot and continue where I left off.

If MemoryOS solves that better than anything else on Windows, it becomes special. If it only adds more commands, users will compare it to Raycast, Flow Launcher, PowerToys Run, Everything, and Windows Search.

## Product Rating Target

Current product direction:

| Area | Current State | Target State |
|---|---:|---:|
| Launcher utility | Strong | Excellent |
| Search coverage | Strong | Excellent |
| Memory experience | Early | Killer |
| AI usefulness | Medium | Context-aware |
| Privacy trust | Needs polish | Core selling point |
| Differentiation | Good idea | Hard-to-copy moat |
| User retention | Unclear | Daily habit |

Target:

> Turn the app from an 8/10 launcher into a 9/10 memory operating layer.

## North Star Metric

Track this above everything else:

> Number of times the user found something they would otherwise have lost.

Secondary metrics:

- Searches that end in opening a result
- Timeline sessions restored
- Clipboard items recovered
- Old files rediscovered
- Projects automatically grouped correctly
- Search refinements needed before success
- Time from query to useful result

## Killer Use Cases

### 1. "What was I working on yesterday?"

This should become the flagship demo.

The app should show:

- Apps used
- Files opened
- Browser pages visited
- Clipboard items copied
- Screenshots taken
- Git repositories touched
- Commits made
- AI-generated session summary

Example output:

```text
Yesterday, you mainly worked on the Tradeo project.

You opened:
- VS Code: Tradeo repository
- Stripe documentation
- payment_controller.rs
- Tradeo_Report.pdf

You copied:
- Stripe webhook payload example
- API route from payment_controller.rs

You searched:
- "stripe webhook rust example"
- "payment dashboard design"

Suggested action:
- Continue Tradeo coding session
```

This one flow is more valuable than dozens of extra commands.

### 2. "Continue my last coding session"

MemoryOS should restore a useful workspace:

- Open project folder
- Open important files
- Reopen relevant browser tabs
- Restore terminal working directory
- Restore window layout where possible
- Show last copied code snippets
- Summarize the previous session

This creates daily stickiness.

### 3. "Find that PDF I opened before class"

The user should be able to search by vague memory:

```text
pdf I opened before class yesterday
```

The app should use:

- File open events
- Time references
- Document content
- Recent folders
- Browser/download history
- Calendar/class context later if available

### 4. "Where did I copy this from?"

Clipboard lineage should answer:

- Source app
- Source file or webpage
- Time copied
- Where it was later pasted, if trackable
- Related project/entity

This is a very strong differentiator because normal clipboard managers do not understand origin and usage.

### 5. "Show everything related to Tradeo"

MemoryOS should automatically create project/entity pages.

An entity page should include:

- Repository
- Files
- Browser tabs
- PDFs
- Screenshots
- Notes
- Clipboard items
- Commits
- Recent sessions
- Related searches
- AI summary

This turns scattered activity into organized memory.

## Product Pillars

### 1. Universal Search

Search should cover:

- Apps
- Files
- Folders
- PDFs
- DOCX files
- Source code
- Images through OCR
- Screenshots
- Browser history
- Bookmarks
- Clipboard history
- Git repositories
- Commits
- Notes
- Settings
- System actions
- Timeline events

Search should support:

- Exact keywords
- Fuzzy matching
- Natural language
- Time references
- Semantic similarity
- Relationship queries
- Recently used context

But search must remain fast. AI should improve ranking and explanations, not replace traditional search.

### 2. Explainable Results

Every important result should explain why it appeared.

Example:

```text
Tradeo_Report.pdf
Opened yesterday at 8:42 PM
Matched: "payment dashboard"
Related to: Tradeo repository, Stripe docs, payment_controller.rs
Why shown: You opened this during yesterday's Tradeo coding session.
```

This builds trust. Without explanations, memory search feels random.

### 3. Timeline Memory

The timeline should show meaningful activity, not noisy raw logs.

Example:

```text
Today

10:12 AM - Opened VS Code: Tradeo
10:18 AM - Opened Stripe documentation
10:25 AM - Copied webhook example
10:37 AM - Edited payment_controller.rs
10:55 AM - Took screenshot of dashboard error
11:04 AM - Searched "stripe webhook signature rust"
11:21 AM - Committed: fix payment webhook handling
```

The timeline should support:

- Search
- Filters
- Project grouping
- App grouping
- Date grouping
- Session summaries
- Continue session

### 4. Entity Graph

MemoryOS should connect related items automatically.

Entity examples:

- Project
- Repository
- Class/course
- Website
- Person
- Document
- Topic
- Task

Relationship examples:

- Opened during same session
- Copied from
- Pasted into
- Mentioned in
- Frequently used together
- Same folder/project
- Same Git repository
- Same browser research trail

The graph does not need to be visually complex at first. It just needs to improve search and project pages.

### 5. Session Restore

Session restore is the feature that makes MemoryOS feel magical.

Minimum useful version:

- Restore browser URLs
- Open project folder
- Open recent files
- Open terminal in previous directory
- Show previous clipboard snippets

Advanced version:

- Restore window layout
- Restore virtual desktop
- Restore editor workspace
- Restore terminal tabs
- Resume focus mode
- Summarize unfinished tasks

## What To Build First

### Phase 1: Make Search Trustworthy

Goal:

> Users should trust MemoryOS more than Windows Search.

Build:

- Unified result ranking
- Result explanations
- Source badges
- Fast search response
- Better filters
- Query history
- Search result feedback

Ship when:

- Searching files, clipboard, browser history, and Git feels fast and predictable.
- Each result clearly explains why it matched.

### Phase 2: Timeline View

Goal:

> Users can rewind their workday.

Build:

- Timeline page
- Daily activity grouping
- App/file/browser/clipboard/git events
- Search inside timeline
- Session detection
- Daily AI summary

Ship when:

- "What was I working on yesterday?" gives a useful answer.

### Phase 3: Continue Session

Goal:

> Users can resume old work in one command.

Build:

- Continue last session
- Continue yesterday's session
- Continue project session
- Restore browser tabs
- Open files/folders
- Open terminal/project
- Show previous clipboard items

Ship when:

- A coding/study/research session can be meaningfully resumed.

### Phase 4: Entity Pages

Goal:

> MemoryOS organizes digital life automatically.

Build:

- Project pages
- Repository pages
- Topic pages
- Related files
- Related searches
- Related clipboard items
- Related screenshots
- Related browser history

Ship when:

- Typing a project name gives a useful dashboard of everything related to it.

### Phase 5: Visual Memory

Goal:

> Users can search images and screenshots by what they remember seeing.

Build:

- Screenshot OCR
- Image OCR
- Basic visual labels
- Search screenshots by text
- Search screenshots by rough description

Ship when:

- Users can find screenshots without remembering filenames.

### Phase 6: Workflows

Goal:

> Users can turn repeated activity into one command.

Build:

- Manual workflow builder
- Trigger phrases
- System action steps
- Open app/file/URL steps
- AI-generated workflow drafts

Ship when:

- Study mode, coding mode, and morning routine are reliable.

### Phase 7: Agents

Goal:

> AI agents use MemoryOS context to do useful work.

Build agents only after memory is strong.

Agents should have:

- Clear permissions
- Approval before risky actions
- Activity logs
- Tool limits
- File access controls
- Per-agent memory
- Kill switch

Agents without memory are just another AI wrapper. Agents with MemoryOS context can become powerful.

## What Not To Prioritize Yet

Avoid spending too much time on:

- More window layouts
- More tiny system commands
- A large extension marketplace
- Fancy AI personas
- Complex visual graph UI
- Too many settings pages
- Custom animation polish before the core works
- Agent autonomy before safety and memory are solved

These can come later. They are not the moat.

## Privacy And Trust Requirements

MemoryOS tracks sensitive data. Trust must be part of the product, not an afterthought.

Required features:

- Local-first storage
- Clear onboarding permission screen
- Pause tracking button
- Private mode
- Exclude apps
- Exclude folders
- Exclude websites
- Delete memory by date
- Delete memory by source
- Delete memory by project/entity
- View what was captured
- Clear AI data controls
- No hidden cloud sync

The app should clearly say:

```text
Your memory stays on your PC by default.
You control what is tracked.
You can pause, exclude, or delete anything.
```

This is critical. Without trust, MemoryOS can feel creepy.

## AI Strategy

AI should not be the product. AI should make memory easier to use.

Good AI uses:

- Summarize a day
- Summarize a session
- Explain why results are related
- Convert vague queries into search filters
- Generate workflow drafts
- Summarize project activity
- Extract tasks from recent work

Bad AI uses:

- Replacing fast search with slow chat
- Making everything dependent on cloud models
- Giving agents too much control too early
- Hiding raw evidence behind AI summaries

Rule:

> Always show the evidence behind AI answers.

Example:

```text
You were working on Tradeo yesterday.

Evidence:
- VS Code active for 2h 14m in Tradeo repo
- Opened Stripe docs 6 times
- Edited payment_controller.rs
- Copied webhook payload example
- Commit: fix payment webhook handling
```



## Recommended App Structure

Main navigation should be simple:

| Section | Purpose |
|---|---|
| Search | Find anything and run actions |
| Timeline | Rewind activity |
| Memory | Browse entities/projects/sources |
| Actions | Commands, workflows, snippets |
| AI | Chat, summaries, agents |
| Settings | Privacy, indexing, models, sources |

Do not expose every command equally. Most users should see outcomes, not implementation categories.

## Better Positioning

Use outcome-first messaging.

Strong:

- Never lose anything on your PC again.
- Your computer finally remembers everything.
- Find what you forgot.
- Rewind your workday.
- Continue where you left off.
- Search files, screenshots, clipboard, browser history, and work sessions from one place.

Weak:

- AI launcher for Windows
- Raycast alternative
- Productivity command bar
- Better Windows Search
- Flow Launcher with AI

## Demo Script

The best product demo should look like this:

### Demo 1: Lost Work

User types:

```text
what was I working on yesterday evening?
```

MemoryOS shows:

- Main project
- Apps used
- Files opened
- Browser docs
- Clipboard items
- Git commits
- Summary
- Continue button

### Demo 2: Vague Search

User types:

```text
that screenshot with the payment error
```

MemoryOS shows the screenshot using OCR/visual memory.

### Demo 3: Clipboard Lineage

User types:

```text
where did I copy this stripe code from?
```

MemoryOS shows the source webpage/file and when it was copied.

### Demo 4: Continue Session

User clicks:

```text
Continue Tradeo session
```

MemoryOS opens:

- Repo
- Important files
- Browser docs
- Terminal
- Session summary

That is the moment users understand the product.

## MVP Definition

The MVP is not complete when it has many commands.

The MVP is complete when these five queries work well:

```text
what was I working on yesterday?
continue my last coding session
find the pdf I opened before class
where did I copy this from?
show everything related to Tradeo
```

If these work, the product has a real identity.

## Quality Bar

MemoryOS must be:

- Fast
- Local-first
- Explainable
- Reliable
- Low idle CPU
- Low memory usage
- Easy to pause
- Easy to delete data from
- Clear about what it tracks

If the memory layer is slow, noisy, or creepy, the product fails.

## Final Strategy

Build fewer features, but make the memory experience unforgettable.

The product should not compete on command count.

It should compete on this:

> No other Windows app understands what I was doing, where my information came from, and how to help me continue.

That is the killer app.

