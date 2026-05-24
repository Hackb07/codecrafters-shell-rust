use std::process::Child;

pub struct Job {
    pub id: usize,
    pub command: String,
    pub child: Child,
}

pub fn reap_jobs(jobs: &mut Vec<Job>) {
    let mut running: Vec<bool> = Vec::with_capacity(jobs.len());
    for job in jobs.iter_mut() {
        running.push(job.child.try_wait().ok() == Some(None));
    }

    let max1 = jobs.iter().map(|j| j.id).max().unwrap_or(0);
    let max2 = jobs
        .iter()
        .filter(|j| j.id != max1)
        .map(|j| j.id)
        .max()
        .unwrap_or(0);

    for (i, &is_running) in running.iter().enumerate() {
        if is_running {
            continue;
        }
        let marker = if jobs[i].id == max1 {
            '+'
        } else if jobs[i].id == max2 {
            '-'
        } else {
            ' '
        };
        let cmd = jobs[i]
            .command
            .trim_end()
            .trim_end_matches('&')
            .trim_end()
            .to_string();
        println!("[{}]{}  {:<24}{}", jobs[i].id, marker, "Done", cmd);
    }

    let mut i = jobs.len();
    while i > 0 {
        i -= 1;
        if !running[i] {
            jobs.remove(i);
        }
    }
}
