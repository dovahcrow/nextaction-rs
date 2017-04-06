# nextaction-rs

Nextaction is a webhook server for Todoist which mimics omnifocus' behavior and can automate your nextaction workflow.

## Details

### @nextaction
Nextaction will auto tag current `nextaction` task with `@nextaction`. It also supports parallel task (with '-' append)
and sequential task (with ':' append).

e.g.
```
|-taskA:
    |-taskB  // This task will be tagged @nextaction
    |-taskC:
        |-taskD
```
after you complete taskB, it will become
```
|-taskA:
    |-taskC:
        |-taskD // This task will be tagged @nextaction
```
And for parallel tasks:
```
|-taskA-
    |-taskB // This task will be tagged @nextaction
    |-taskC // This task will also be tagged @nextaction
```

So that you can add a filter on @nextaction to make you focused.

Parallel tasks and sequential tasks can corporate with each other seamlessly:
```
|-taskA-
    |-taskB:
    |   |-taskC // This task will be tagged @nextaction
    |   |-taskD
    |-taskE // This task will be tagged @nextaction
```

### @someday
Nextaction also supports a tag called `@someday`. The logic is:
when Nextaction meets a task which should be tagged `@nextaction`
but currently has tag `@someday`, it won't tag `@nextaction` to that task.
So that your someday tasks won't show up on your nextaction list.

## Usage
You should set environment variable `NXTT_token` to your todoist token.

To build the application, (rust)[`rustup.rs`] is needed.
Run: `git clone && cargo run --release`

### Docker Image
A docker image is also available. Run it by `docker run -it -e NXTT_token=<your todoist token> wooya/nextaction`

## Todo
- [ ] Make nextaction-rs cargo-installable
- [ ] Auto complete parent task && archive parent project if all sub tasks/projects are completed/archived
- [ ] Add auto review system
