# Accessibility

Peisar is an open-source Markdown parser and renderer. We want its
documentation, examples, APIs, and generated HTML to be usable by people with
disabilities and by people using keyboards, screen readers, magnification, or
other assistive technology.

Because Peisar is a library, the accessibility of a final website or
application also depends on the Markdown content, styles, scripts, and user
interface provided by the application that uses it. This document describes
what the project maintains and how to report a barrier.

## Our commitments

We aim to:

- write documentation and examples in clear language and use meaningful
  headings, links, and code samples;
- keep project documentation usable with keyboard navigation and assistive
  technology where the hosting platform supports it;
- preserve meaningful Markdown structure when rendering HTML, including
  headings, lists, tables, links, and alternative text supplied for images;
- avoid changes that unnecessarily remove semantic HTML or make generated
  output harder for consuming applications to make accessible; and
- consider accessibility impact when reviewing user-facing documentation,
  examples, and rendering changes.

We do not claim that Peisar, its documentation, or HTML produced from arbitrary
Markdown conforms to a particular accessibility standard. Applications using
Peisar remain responsible for evaluating their complete user experience.

## Guidance for users of Peisar

When using Peisar to render Markdown in an application, use the generated HTML
as semantic content rather than replacing its structure with non-semantic
elements. In particular:

- use one logical heading hierarchy;
- provide useful alternative text for informative images;
- ensure links describe their destination or purpose;
- give tables headers and avoid using tables only for visual layout;
- provide captions, transcripts, or other alternatives for audio and video;
- make custom controls and interactive content keyboard-operable; and
- test the rendered page with the browsers and assistive technology used by
  your audience.

The repository's examples demonstrate integration, not a guarantee that the
example applications meet every accessibility need.

## Contributor expectations

Contributors should consider accessibility when changing documentation,
examples, public APIs, or HTML rendering. User-facing changes should preserve
semantic structure and should not rely only on color, pointer input, or visual
position to convey information or complete a task.

When a change affects rendered HTML or an example application, describe any
accessibility impact in the pull request and include appropriate tests or
manual verification. If a trade-off is necessary, explain it so maintainers
and users can evaluate it.

## Reporting an accessibility issue

Please report accessibility barriers through the
[GitHub issue tracker](https://github.com/phothinmg/peisar/issues). You do not
need to disclose a disability or provide a screenshot, recording, or personal
information.

Useful details include:

- the task you were trying to complete;
- the Markdown input, documentation page, example, or API involved;
- what happened and what you expected instead;
- the Peisar version and, if relevant, your browser or Node.js version; and
- the operating system, browser, and assistive technology involved, if you are
  comfortable sharing them.

If possible, include a small reproducible Markdown example. Please remove any
private or sensitive information first.

## How we respond

Maintainers will triage reported barriers, request only the information needed
to reproduce them, and track confirmed issues publicly when appropriate. We
will prioritize barriers that prevent someone from reading documentation,
using the library, or completing a core task. We cannot promise a specific
resolution time, but we will provide status updates as investigation and fixes
progress.

If you can suggest or verify a fix, contributions and follow-up feedback are
welcome.

## Improving this statement

Accessibility practices evolve with the project and its users. To suggest an
improvement to this statement or to the project's accessibility approach, open
an issue in the [GitHub issue tracker](https://github.com/phothinmg/peisar/issues).
For an active accessibility barrier, use the reporting process above so it can
be triaged promptly.
