use anyhow::{bail, Result};
use std::collections::HashSet;

/// Maximum dependency-chain depth `walk_task_graph` will descend.
///
/// The walk recurses once per level, so an unbounded chain exhausts the thread
/// stack and aborts the process (SIGABRT, exit 134) — a crash no caller can
/// catch or report. This bound turns that into an ordinary `Err`. It sits far
/// under the depth a debug build actually overflows at (~16,000, and CI builds
/// debug), so the guard is what a deep graph meets, not the stack. No real task
/// graph approaches this; a chain this long is a generated or malformed
/// `fledge.toml`.
pub(crate) const MAX_TASK_DEPTH: usize = 1_000;

/// Depth-first walk of a task dependency graph.
///
/// Two sets keep diamond DAGs (two tasks sharing one dep) from being reported
/// as cycles:
///
/// - `in_progress` is the recursion stack. Insert before walking deps, remove
///   after returning. A hit here is a genuine cycle; the error lists that
///   ordered walk (not a `HashSet` iteration).
/// - `completed` is nodes whose subgraph has already been walked. A hit here
///   returns `Ok` immediately so shared deps are not re-run.
///
/// `on_ready` is invoked once per node after its dependencies, just before the
/// node is marked completed.
pub(crate) fn walk_task_graph<'graph, Deps, Ready>(
    name: &str,
    deps_of: &Deps,
    in_progress: &mut Vec<String>,
    completed: &mut HashSet<String>,
    on_ready: &mut Ready,
) -> Result<()>
where
    Deps: Fn(&str) -> Option<&'graph [String]>,
    Ready: FnMut(&str) -> Result<()>,
{
    if completed.contains(name) {
        return Ok(());
    }
    if let Some(idx) = in_progress.iter().position(|n| n == name) {
        let mut cycle: Vec<&str> = in_progress[idx..].iter().map(String::as_str).collect();
        cycle.push(name);
        bail!("Circular dependency detected: {}", cycle.join(" → "));
    }

    if in_progress.len() >= MAX_TASK_DEPTH {
        bail!(
            "Dependency chain deeper than {MAX_TASK_DEPTH} tasks (reached '{name}') — \
             check task deps for an unintended chain"
        );
    }

    in_progress.push(name.to_string());
    for dep in deps_of(name).unwrap_or(&[]) {
        walk_task_graph(dep, deps_of, in_progress, completed, on_ready)?;
    }
    on_ready(name)?;
    in_progress.pop();
    completed.insert(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn graph(edges: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        edges
            .iter()
            .map(|(name, deps)| {
                (
                    (*name).to_string(),
                    deps.iter().map(|d| (*d).to_string()).collect(),
                )
            })
            .collect()
    }

    fn walk(g: &BTreeMap<String, Vec<String>>, start: &str) -> Result<Vec<String>> {
        let mut ready = Vec::new();
        walk_task_graph(
            start,
            &|n| g.get(n).map(Vec::as_slice),
            &mut Vec::new(),
            &mut HashSet::new(),
            &mut |n| {
                ready.push(n.to_string());
                Ok(())
            },
        )?;
        Ok(ready)
    }

    /// A linear chain `t0 → t1 → … → t{len-1}`, so walking `t0` descends `len`
    /// levels. The shape that overflowed the stack before `MAX_TASK_DEPTH`.
    fn chain(len: usize) -> BTreeMap<String, Vec<String>> {
        (0..len)
            .map(|i| {
                let deps = if i + 1 < len {
                    vec![format!("t{}", i + 1)]
                } else {
                    Vec::new()
                };
                (format!("t{i}"), deps)
            })
            .collect()
    }

    #[test]
    fn diamond_is_not_a_cycle_and_shared_dep_runs_once() {
        // a → [b, c], b → [d], c → [d]
        let g = graph(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        let ready = walk(&g, "a").expect("diamond DAG must not be reported as a cycle");
        assert_eq!(ready, vec!["d", "b", "c", "a"]);
        assert_eq!(ready.iter().filter(|n| *n == "d").count(), 1);
    }

    #[test]
    fn two_cycle_reports_ordered_path() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        let err = walk(&g, "a").unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency detected: a → b → a"),
            "expected ordered cycle walk, got: {err}"
        );
    }

    #[test]
    fn self_cycle_reports_ordered_path() {
        let g = graph(&[("a", &["a"])]);
        let err = walk(&g, "a").unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency detected: a → a"),
            "expected self-cycle walk, got: {err}"
        );
    }

    #[test]
    fn longer_cycle_reports_actual_cycle_not_prefix() {
        // x is on the path to the cycle but not on the cycle itself
        let g = graph(&[("x", &["a"]), ("a", &["b"]), ("b", &["c"]), ("c", &["a"])]);
        let err = walk(&g, "x").unwrap_err().to_string();
        assert!(
            err.contains("Circular dependency detected: a → b → c → a"),
            "expected the cycle itself as an ordered walk, got: {err}"
        );
        assert!(
            !err.contains("x →"),
            "cycle path must not include the acyclic prefix, got: {err}"
        );
    }

    #[test]
    fn detect_cycle_ok_on_diamond() {
        let g = graph(&[("a", &["b", "c"]), ("b", &["d"]), ("c", &["d"]), ("d", &[])]);
        walk(&g, "a").unwrap();
    }

    #[test]
    fn detect_cycle_err_on_loop() {
        let g = graph(&[("a", &["b"]), ("b", &["a"])]);
        let err = walk(&g, "a").unwrap_err().to_string();
        assert!(err.contains("a → b → a"), "got: {err}");
    }

    #[test]
    fn chain_at_the_depth_limit_walks() {
        let g = chain(MAX_TASK_DEPTH);
        let ready = walk(&g, "t0").expect("a chain exactly at the bound must still walk");
        assert_eq!(ready.len(), MAX_TASK_DEPTH);
    }

    #[test]
    fn chain_one_past_the_depth_limit_bails() {
        let g = chain(MAX_TASK_DEPTH + 1);
        let err = walk(&g, "t0").unwrap_err().to_string();
        assert!(
            err.contains("Dependency chain deeper than"),
            "expected the depth bound to be named, got: {err}"
        );
    }

    /// The regression #513's test plan claimed but never landed. Before the
    /// bound this aborted the whole test binary with a stack overflow rather
    /// than failing; with it, the walk never descends past `MAX_TASK_DEPTH`.
    #[test]
    fn deep_chain_bails_instead_of_overflowing_the_stack() {
        let g = chain(20_000);
        let err = walk(&g, "t0").unwrap_err().to_string();
        assert!(
            err.contains(&format!("deeper than {MAX_TASK_DEPTH} tasks")),
            "expected a catchable depth error, got: {err}"
        );
    }

    #[test]
    fn depth_is_nesting_not_total_nodes() {
        // 2,000 tasks, but a shallow graph: one root with 1,999 leaf deps.
        // Only nesting is bounded, so this must walk cleanly.
        let mut g = BTreeMap::new();
        let leaves: Vec<String> = (1..2_000).map(|i| format!("t{i}")).collect();
        for leaf in &leaves {
            g.insert(leaf.clone(), Vec::new());
        }
        g.insert("t0".to_string(), leaves);
        let ready = walk(&g, "t0").expect("a wide but shallow graph must not hit the bound");
        assert_eq!(ready.len(), 2_000);
    }
}
