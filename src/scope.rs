//! Logging scope and Scope.
use crate::raw_events::{RayonEvent, TaskId};
use crate::{pool::log, pool::next_task_id};
use std::mem::transmute;
use time::precise_time_ns;

///Represents a fork-join scope which can be used to spawn any number of tasks. See [`scope()`] for more information.
///
///[`scope()`]: fn.scope.html
pub struct Scope<'scope> {
    rayon_scope: Option<&'scope rayon::Scope<'scope>>,
    continuing_task_id: TaskId,
}

impl<'scope> Scope<'scope> {
    /// Spawns a job into the fork-join scope `self`. This job will
    /// execute sometime before the fork-join scope completes.  The
    /// job is specified as a closure, and this closure receives its
    /// own reference to the scope `self` as argument. This can be
    /// used to inject new jobs into `self`.
    ///
    /// # Returns
    ///
    /// Nothing. The spawned closures cannot pass back values to the
    /// caller directly, though they can write to local variables on
    /// the stack (if those variables outlive the scope) or
    /// communicate through shared channels.
    ///
    /// (The intention is to eventualy integrate with Rust futures to
    /// support spawns of functions that compute a value.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut value_a = None;
    /// let mut value_b = None;
    /// let mut value_c = None;
    /// rayon::scope(|s| {
    ///     s.spawn(|s1| {
    ///           // ^ this is the same scope as `s`; this handle `s1`
    ///           //   is intended for use by the spawned task,
    ///           //   since scope handles cannot cross thread boundaries.
    ///
    ///         value_a = Some(22);
    ///
    ///         // the scope `s` will not end until all these tasks are done
    ///         s1.spawn(|_| {
    ///             value_b = Some(44);
    ///         });
    ///     });
    ///
    ///     s.spawn(|_| {
    ///         value_c = Some(66);
    ///     });
    /// });
    /// assert_eq!(value_a, Some(22));
    /// assert_eq!(value_b, Some(44));
    /// assert_eq!(value_c, Some(66));
    /// ```
    ///
    /// # See also
    ///
    /// The [`scope` function] has more extensive documentation about
    /// task spawning.
    ///
    /// [`scope` function]: fn.scope.html
    pub fn spawn<BODY>(&self, body: BODY)
    where
        BODY: FnOnce(&Scope<'scope>) + Send + 'scope,
    {
        let spawned_id = next_task_id();
        let seq_id = next_task_id();
        logs!(RayonEvent::Child(spawned_id), RayonEvent::Child(seq_id));
        // sorry I need to erase the borrow's lifetime.
        // it's ok though since the pointed self will survive all spawned tasks.
        let floating_self: &'scope Scope<'scope> = unsafe { transmute(self) };
        let logged_body = move |_: &rayon::Scope<'scope>| {
            log(RayonEvent::TaskStart(spawned_id, precise_time_ns()));
            body(floating_self);
            logs!(
                RayonEvent::Child(floating_self.continuing_task_id),
                RayonEvent::TaskEnd(precise_time_ns())
            );
        };
        self.rayon_scope.as_ref().unwrap().spawn(logged_body);
        logs!(
            RayonEvent::TaskEnd(precise_time_ns()),
            RayonEvent::TaskStart(seq_id, precise_time_ns())
        );
    }
}

/// Create a "fork-join" scope `s` and invokes the closure with a
/// reference to `s`. This closure can then spawn asynchronous tasks
/// into `s`. Those tasks may run asynchronously with respect to the
/// closure; they may themselves spawn additional tasks into `s`. When
/// the closure returns, it will block until all tasks that have been
/// spawned into `s` complete.
pub fn scope<'scope, OP, R>(op: OP) -> R
where
    OP: for<'s> FnOnce(&'s Scope<'scope>) -> R + 'scope + Send,
    R: Send,
{
    let scope_id = next_task_id();
    let continuing_task_id = next_task_id();
    logs!(
        RayonEvent::Child(scope_id),
        RayonEvent::TaskEnd(precise_time_ns())
    );
    // the Scope structure needs to survive the scope fn call
    // because tasks might be executed AFTER the op call completed
    let mut borrowed_scope: Scope<'scope> = Scope {
        rayon_scope: None, // we cannot know now so we use a None
        continuing_task_id,
    };
    let borrowed_scope_ref = &mut borrowed_scope;
    let r = rayon::scope(move |s| {
        log(RayonEvent::TaskStart(scope_id, precise_time_ns()));
        // I'm sorry, there is no other way to do it without changing
        // the API. Because I can only access a reference to the underlying rayon::Scope
        borrowed_scope_ref.rayon_scope = unsafe { transmute(Some(s)) };
        let r = op(borrowed_scope_ref);
        logs!(
            RayonEvent::Child(continuing_task_id),
            RayonEvent::TaskEnd(precise_time_ns())
        );
        r
    });
    log(RayonEvent::TaskStart(continuing_task_id, precise_time_ns()));
    r
}
