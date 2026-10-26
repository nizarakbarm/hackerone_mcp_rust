# Scan brief

The `agent: scout` entry in step 1 walks the codebase with this brief inlined (scouts start blank):

## Brief

> Explore these paths: <scoped paths from step 1>.
>
> Don't follow rigid heuristics; explore organically and note where you experience friction:
>
> - Where does understanding one concept require bouncing between many small modules?
> - Where are modules **shallow**, with an interface nearly as complex as the implementation?
> - Where have pure functions been extracted just for testability, but the real bugs hide in how they're called (no **locality**)?
> - Where do tightly-coupled modules leak across their seams?
> - Which parts of the codebase are untested, or hard to test through their current interface?
>
> Apply the **deletion test** to anything you suspect is shallow: would deleting it concentrate complexity, or just move it? A "yes, concentrates" is the signal.
>
> Report friction with file paths and evidence. Do not propose interfaces, and do not edit anything.

Vocabulary to use in your report (**module**, **interface**, **depth**, **seam**, **adapter**, **leverage**, **locality**): `skill://codebase-design`.
