# BOY-000: Design Documentation Workflow

## Document Information

| Field               | Value                                                |
| ------------------- | ---------------------------------------------------- |
| **Document Number** | BOY-000                                              |
| **Date**            | 2026-11-20                                           |
| **Author**          | João Magalhães <joamag@gmail.com>                    |
| **Subject**         | Design Documentation Process and Workflow Guidelines |
| **Status**          | Active                                               |
| **Version**         | 1.0                                                  |

## Description

### Purpose

Design documents provide a structured way to document technical decisions, solutions, and architectural changes in Boytacean. They serve as a foundation for future technical documentation and help maintain the knowledge about why certain design choices were made. This document defines the workflow and guidelines for creating and maintaining design documents.

### When to Create a Design Document

Create a design document when implementing:

- Emulation of a new system or of a new hardware component
- Significant architectural changes or refactoring of the emulation core
- New features that affect several front-ends (SDL, web, libretro, Python)
- New file formats or changes to existing ones (eg: save states)
- Integration with external hardware or services
- Performance optimizations with trade-offs

Simple bug fixes, minor tweaks, and straightforward feature additions typically do not require design documents. Reference documentation that describes something without a design decision behind it (eg: a hardware protocol) stays directly in `doc/`.

### Document Structure

All design documents must include a **Document Information** table with the following fields:

- **Document Number**: Use format `BOY-XXX` where XXX is a zero-padded sequential number (e.g., BOY-001, BOY-002)
- **Date**: Document creation date in ISO format (YYYY-MM-DD)
- **Author**: Full name and email address
- **Subject**: Brief title describing the design topic
- **Status**: Current document status (see Status Values below)
- **Version**: Document version number (e.g., 1.0, 1.1, 2.0)

The **Description** section should be concise (3-5 minute read) and include:

- **Problem**: What problem is being solved and why
- **Solution**: How the solution works and key technical details
- **Usage**: Practical examples showing how to use or configure the feature

Additional sections may be added as needed for complex topics, but keep documents focused and readable.

### Document Numbering

Documents are numbered sequentially starting from BOY-001. BOY-000 is reserved for this workflow document. Check the `doc/design/` directory to find the next available number. Document numbers are never reused, even if a document becomes obsolete.

### Status Values

Documents use the following status values:

- **Draft**: Document is being written and not yet reviewed
- **Under Review**: Document is complete and awaiting feedback
- **Approved**: Document has been reviewed and approved
- **Active**: Document describes a workflow or process that is currently in use
- **Implemented**: Document describes a feature that has been implemented
- **Superseded**: Document has been replaced by a newer document (reference the new document number)
- **Obsolete**: Document describes a feature or approach no longer in use

### File Location and Naming

Design documents must be stored in `doc/design/` with the naming convention:

```text
XXX-short_descriptive_name.md
```

For example:

- `000-design_workflow.md`
- `001-gba_performance.md`

Use lowercase with underscores for the descriptive portion. Keep filenames concise but meaningful. List every document in the index of `doc/design/README.md`.

### Review Process

1. Create document with status **Draft** and open a pull request
2. Update status to **Under Review** when ready for feedback
3. Address review comments and update version number for significant changes
4. Once approved, update status to **Approved**, **Active**, or **Implemented** as appropriate
5. Merge the pull request

For workflow documents like this one, use status **Active**. For feature implementations, use **Implemented**.

### Maintenance

Update documents when:

- Implementation details change significantly (increment minor version, e.g., 1.0 to 1.1)
- The design approach changes fundamentally (create new document and mark old as **Superseded**)
- A feature is removed or deprecated (update status to **Obsolete**)

Always preserve the original document structure when updating. Add change notes at the end if tracking significant revisions.

---

**Document Classification**: Public Technical Documentation
