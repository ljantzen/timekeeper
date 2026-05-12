use anyhow::Result;
use tmkpr_lib::service::CommentService;
use tmkpr_lib::storage::Storage;

use crate::cli::{CommentAddArgs, CommentDeleteArgs, CommentEditArgs, CommentListArgs};
use crate::output;
use crate::prompt;

pub fn add(args: CommentAddArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let body = args.body.join(" ");
    let comment = CommentService::new(storage, user_id).add(body)?;
    println!(
        "Added comment {} to active entry.",
        &comment.id[..comment.id.len().min(8)]
    );
    Ok(())
}

pub fn list(
    args: CommentListArgs,
    storage: &dyn Storage,
    user_id: &str,
    date_fmt: &str,
    format: &str,
) -> Result<()> {
    let comments = CommentService::new(storage, user_id).list(args.entry.as_deref())?;
    output::print_comments(&comments, date_fmt, format);
    Ok(())
}

pub fn edit(args: CommentEditArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let body = args.body.join(" ");
    let comment = CommentService::new(storage, user_id).edit(&args.id, body)?;
    println!(
        "Updated comment {}.",
        &comment.id[..comment.id.len().min(8)]
    );
    Ok(())
}

pub fn delete(args: CommentDeleteArgs, storage: &dyn Storage, user_id: &str) -> Result<()> {
    let svc = CommentService::new(storage, user_id);

    if !args.yes
        && !prompt::confirm(&format!(
            "Delete comment {}?",
            &args.id[..args.id.len().min(8)]
        ))
    {
        println!("Cancelled.");
        return Ok(());
    }

    svc.delete(&args.id)?;
    println!("Deleted comment {}.", &args.id[..args.id.len().min(8)]);
    Ok(())
}
