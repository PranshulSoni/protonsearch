# Product Vision: MemoryOS (Working Concept)

## Executive Summary

This project is **NOT** intended to be another launcher or a clone of
Raycast, Flow Launcher, RustCast, or PowerToys Run.

The launcher is only the **entry point**.

The actual product is a **Windows Intelligence Layer** that continuously
builds a searchable understanding of the user's digital life.

Core mission:

> Never lose anything on your computer again.

Secondary mission:

> Search anything. Do anything. Continue everything.

------------------------------------------------------------------------

# Product Positioning

Do NOT position the product as:

-   AI Launcher
-   Raycast Alternative
-   Better Flow Launcher
-   Windows Search Replacement

Instead position it as:

-   The memory layer for Windows
-   Your computer finally remembers everything
-   The operating system Windows should have had

Users should install it because they constantly lose information, not
because they want another launcher.

------------------------------------------------------------------------

# Core Product Pillars

## Universal Search

One search box for:

-   Applications
-   Files
-   Folders
-   PDFs
-   Office documents
-   Images
-   OCR text
-   Browser history
-   Bookmarks
-   Open tabs
-   Clipboard history
-   Git repositories
-   Commits
-   Functions
-   Classes
-   Notes
-   Windows settings
-   Control Panel
-   System actions

Search should support:

-   Keywords
-   Natural language
-   Time references
-   Context
-   Relationships
-   Semantic similarity

------------------------------------------------------------------------

## Computer Memory

Instead of indexing only files, continuously record meaningful events.

Examples:

-   Opened a PDF
-   Copied text
-   Edited a document
-   Opened a repository
-   Visited a website
-   Took a screenshot

This enables queries like:

-   What was I working on yesterday?
-   Which PDF did I open before class?
-   What code did I copy last week?
-   Continue what I was doing before dinner.

------------------------------------------------------------------------

## Universal Actions

The search bar is also an action bar.

Examples:

-   Enable Bluetooth
-   Restart Explorer
-   Set volume to 40%
-   Compress folder
-   Convert PDF
-   Open startup folder
-   Shutdown PC
-   Deploy project

Users search for outcomes instead of applications.

------------------------------------------------------------------------

## Workflow Automation

Support reusable workflows.

Examples:

-   Start development environment
-   Deploy project
-   Morning routine
-   Research workflow

Future:

-   Visual workflow builder
-   Variables
-   Conditions
-   Scheduling
-   Auto-generated workflows based on repeated behavior

------------------------------------------------------------------------

# Differentiating Features

## Timeline Rewind

Restore a previous workspace.

Restore:

-   Browser tabs
-   Explorer windows
-   Editors
-   Terminal sessions
-   Window layout

------------------------------------------------------------------------

## Clipboard Lineage

Track where copied information came from and where it was eventually
used.

------------------------------------------------------------------------

## Entity Graph

Automatically connect related information into project entities.

Example:

Tradeo

-   Repository
-   PDFs
-   Browser tabs
-   Notes
-   Screenshots
-   Commits
-   Clipboard entries

------------------------------------------------------------------------

## Relationship Search

Support searches like:

-   File opened before this one
-   Screenshot after meeting
-   Note related to repository
-   PDF connected to project

------------------------------------------------------------------------

## Explain Results

Every result should explain WHY it matched.

Example:

-   Opened yesterday
-   Mentioned in README
-   Frequently used
-   Related to Tradeo

------------------------------------------------------------------------

## Search by Visual Memory

Support queries such as:

-   Blue graph
-   Three tables
-   Dark themed document
-   Screenshot with terminal

Use OCR + vision features.

------------------------------------------------------------------------

## Intent Search

Users search for goals instead of software.

Example:

"Pay electricity bill"

System finds the best app, website, workflow or document.

------------------------------------------------------------------------

## Session Intelligence

Automatically detect:

-   Coding sessions
-   Study sessions
-   Meetings
-   Research sessions

Allow searching and restoring entire sessions.

------------------------------------------------------------------------

## Causal History

Store chains of actions instead of isolated events.

Example:

Opened PDF → Copied paragraph → Pasted into Notes → Referenced in code →
Git commit

------------------------------------------------------------------------

# Search Architecture

Use hybrid retrieval.

Pipeline:

Metadata + Full Text Search + Semantic Search + Ranking

Embeddings should only improve ranking---not replace search.

------------------------------------------------------------------------

# Data Pipeline

Watchers:

-   Filesystem
-   Browser
-   Clipboard
-   Git
-   Screenshots
-   Applications

Pipeline:

Watchers → Event Store → Full Text Index → Vector Index → Entity Graph →
Search Engine

------------------------------------------------------------------------

# Performance Goals

Current prototype:

\~50 MB RAM

Targets:

Idle: 40--80 MB

Searching: 80--150 MB

Heavy indexing: 150--300 MB (temporary)

Requirements:

-   Fast startup
-   Near-zero idle CPU
-   CPU throttled indexing
-   Battery-aware behavior
-   Disk-first architecture
-   SQLite + FTS + memory-mapped indexes
-   Incremental indexing
-   Do not embed everything

------------------------------------------------------------------------

# Embedding Strategy

Never generate embeddings for all files.

Preferred strategy:

-   Full-text index everything useful
-   Embed only important or frequently used documents
-   Generate embeddings on demand when beneficial
-   Cache generated embeddings

Embeddings are an enhancement layer.

------------------------------------------------------------------------

# Long-Term Moat

Competitors can copy:

-   UI
-   Launcher
-   Commands
-   Animations

Competitors cannot easily copy:

-   Years of activity history
-   Personal knowledge graph
-   Learned workflows
-   Context relationships
-   Memory timeline

The product becomes more valuable every day it is installed.

------------------------------------------------------------------------

# Marketing Strategy

Never market features first.

Market outcomes.

Primary messages:

-   Never lose anything on your PC again.
-   Your computer finally remembers everything.
-   Search anything. Do anything. Continue everything.
-   Find what you forgot.

The launcher should be presented as the interface---not the product.

------------------------------------------------------------------------

# Guiding Principles

-   Search infrastructure is the foundation.
-   AI enhances the experience but is not the product.
-   Performance is a feature.
-   Privacy and local-first are mandatory.
-   Every feature should reduce friction or restore forgotten context.
-   The memory graph is the long-term competitive advantage.
