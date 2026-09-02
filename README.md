<h1 align="center">cmux-herdr</h1>
<p align="center"><strong>A cmux plugin for Herdr</strong></p>
<p align="center">
  Live status pills, real tabs and splits, and a CLI that treats nested
  Herdr agents as first-class cmux surfaces.
</p>

<p align="center">
  <a href="https://github.com/RaviTharuma/cmux-herdr/actions/workflows/ci.yml"><img src="https://github.com/RaviTharuma/cmux-herdr/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/RaviTharuma/cmux-herdr/releases/latest"><img src="https://img.shields.io/github/v/release/RaviTharuma/cmux-herdr" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License" /></a>
  <a href="https://www.python.org/downloads/"><img src="https://img.shields.io/badge/python-3.10%2B-blue.svg" alt="Python 3.10+" /></a>
  <a href="https://github.com/topics/plugin"><img src="https://img.shields.io/badge/kind-cmux%20plugin-4c71f2.svg" alt="cmux plugin" /></a>
</p>

<p align="center">
  English ·
  <a href="docs/de/README.md">Deutsch</a>
  ·
  <a href="https://github.com/manaflow-ai/cmux">cmux</a>
  ·
  <a href="https://github.com/herdrdev/herdr">Herdr</a>
  ·
  <a href="CHANGELOG.md">changelog</a>
</p>

**cmux-herdr** is the plugin you install when [Herdr](https://github.com/herdrdev/herdr)
runs *inside* [cmux](https://github.com/manaflow-ai/cmux). cmux is the outer
macOS terminal. Herdr is the inner agent mux. Without this plugin, every agent
collapses into one cmux tab titled roughly `herdr`. With it, each pane gets a
status pill in the cmux sidebar, each tab can become a real cmux surface, and you drive both layers
from one CLI: `cmux-herdr`.

This is a released plugin (**v0.4.0**), not a patch to `cmux.app`. Python 3.10+,
standard library only — no `pip`, no `npm`, no compile step.
