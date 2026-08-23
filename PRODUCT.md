# Product

## Platform

web (desktop works through the same renderer-agnostic APIs)

## Users

Rust developers building Dioxus applications that need an editor-style
workbench — IDEs, studios, dashboards, operations consoles, data tools. They
embed `dioxus-workbench` as a crate and render their own panel content inside
it. Many arrive knowing VS Code's shell and expect its concepts to exist
here: tabbed groups, drag-to-dock, an activity bar, a status bar.

## Product Purpose

A dockable panel workbench for Dioxus: tabbed panel groups with drag-to-dock
and resizable splits, plus the surrounding application chrome (activity rail,
status bar). Success is becoming the go-to workbench answer in the Dioxus
ecosystem — crates.io publication, adoption, documentation, and examples all
matter.

## Positioning

The VS Code-workbench experience in pure Rust, without paying the cost of a
JavaScript stack. No `web-sys`, no JS dependencies: measurement, pointer
capture, and focus go through Dioxus's own document APIs, so the same crate
serves web and desktop. The application owns content and meaning; the crate
owns arrangement, interaction states, persistence format, and chrome.

## Non-goals

- An icon set, button system, or general component library. Panel content is
  application territory; the crate ships only the chrome it draws itself.
- Storage. Layouts encode to a string; where it lives is the application's
  decision.
- Floating/undocked windows (roadmap, not scope today).
