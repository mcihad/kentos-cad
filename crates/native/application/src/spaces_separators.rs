//! One semantic plan/execute path for explicit room separator roles (ADR 0679).
use crate::{ExecutionContext,checks,codes,polyline,spaces};
use crate::checks::Stop;
use kentos_commands::*;
use kentos_domain::{Document,Slot};
const STEP:&str="Mahal ayırıcı";
struct Work {entities:Vec<Entity>,slots:Option<Vec<Slot>>,warnings:Vec<CommandWarning>}
fn fail(words:impl Into<String>,path:&str)->Stop {
    spaces::fail(codes::INVALID_SPACE_SEPARATOR,words.into(),path)
}
fn role_valid(p:&PathEntity,path:&str)->Result<(),Stop> {
    if let Some(why)=p.extras_problem(false){return Err(fail(why,path));}Ok(())
}
fn operation_error(mut why:Stop)->Stop {
    match &mut why {
        Stop::Failed(error)|Stop::Conflict(error)=>if let Some(path)=&mut error.path {*path=format!("operation.{path}");},
        Stop::Cancelled=>{},
    } why
}
fn check(doc:&Document,input:&SpacesSeparators)->Result<Work,Stop> {
    checks::revision(doc,input.expected_revision.as_deref())?;
    let mut entities=Vec::new();let mut warnings=Vec::new();
    let mut add_warnings=|more:Vec<CommandWarning>|for mut warning in more {
        if let Some(path)=&mut warning.path && !path.starts_with("operation.") {*path=format!("operation.{path}");}
        if !warnings.contains(&warning){warnings.push(warning);}
    };
    let slots=match &input.operation {
        SpaceSeparatorOperation::Create {layer_id,paths,color,line_weight,line_type,attrs}=> {
            if paths.is_empty() || paths.len()>MAX_SEPARATOR_PATHS
                || paths.iter().try_fold(0usize,|n,p|n.checked_add(p.pts.len())).is_none_or(|n|n>MAX_SEPARATOR_VERTICES)
            {return Err(fail("Bir çağrı 1–10000 ayırıcı ve toplam en çok 100000 köşe içerebilir.","operation.paths"));}
            for (i,path) in paths.iter().enumerate() {
                let at=format!("operation.paths[{i}]");
                let arch=path.arch.clone().unwrap_or_default();
                spaces::level_known(doc,&arch,&format!("{at}.arch.level"))?;
                let polyline=PolylineCreate {layer_id:layer_id.clone(),pts:path.pts.clone(),bulges:path.bulges.clone(),closed:path.closed,
                    widths:None,color:color.clone(),line_weight:*line_weight,line_type:Some(line_type.unwrap_or(kentos_commands::layer::LineType::Dashed)),
                    attrs:attrs.clone(),label:None,symbol:None,expected_revision:None};
                let (output,more)=match polyline::plan_in(doc,&polyline) {
                    CommandResult::Completed {output,warnings}=>(output,warnings),
                    CommandResult::Failed {mut error}|CommandResult::Conflict {mut error}|CommandResult::NeedsInput {mut error}=> {
                        if let Some(field)=&mut error.path {
                            *field=if ["layerId","lineWeight","color"].contains(&field.as_str()) || field.starts_with("attrs") {
                                format!("operation.{field}")
                            } else {format!("{at}.{field}")};
                        }
                        return Err(Stop::failed(error));
                    }
                    _=>return Err(fail("Ayırıcı yolu hazırlanamadı.",&at)),
                };
                add_warnings(more);
                let Entity::Polyline(mut p)=output.entity else {return Err(fail("Ayırıcı çoklu çizgi olmalı.",&at));};
                p.space_separator=Some(arch);role_valid(&p,&at)?;
                entities.push(Entity::Polyline(p));
            }
            None
        }
        SpaceSeparatorOperation::Set {uids,..}|SpaceSeparatorOperation::Clear {uids}=> {
            checks::uids(uids,"Mahal ayırıcı değiştirilemedi.").map_err(operation_error)?;
            if uids.len()>MAX_SEPARATOR_PATHS {return Err(fail("Bir çağrıda en çok 10000 ayırıcı düzenlenebilir.","operation.uids"));}
            let named=checks::named(doc,uids).map_err(operation_error)?;let mut slots=Vec::with_capacity(named.len());
            for (at,slot,e,_) in named {
                let path=format!("operation.uids[{at}]");
                let Entity::Polyline(p)=e else {return Err(fail("Mahal ayırıcı rolü yalnız çoklu çizgide kullanılabilir.",&path));};
                add_warnings(checks::layer_at(doc,&p.base.layer_id,&path)?);
                let mut p=p.clone();
                p.space_separator=match &input.operation {
                    SpaceSeparatorOperation::Set {arch,..}=> {
                        let arch=arch.clone().unwrap_or_default();spaces::level_known(doc,&arch,"operation.arch.level")?;Some(arch)
                    }
                    _=>None,
                };
                role_valid(&p,&path)?;slots.push(slot);entities.push(Entity::Polyline(p));
            }
            Some(slots)
        }
    };
    Ok(Work {entities,slots,warnings})
}
pub fn plan_in(doc:&Document,input:&SpacesSeparators)->CommandResult<SpaceSeparatorsPlan> {
    match check(doc,input) {
        Ok(work)=>CommandResult::Completed {output:SpaceSeparatorsPlan {entities:work.entities,revision:doc.revision().to_string()},warnings:work.warnings},
        Err(why)=>why.into(),
    }
}
pub fn plan(cx:&ExecutionContext<'_>,input:&SpacesSeparators)->CommandResult<SpaceSeparatorsPlan> {plan_in(cx.doc,input)}
pub fn validate(cx:&ExecutionContext<'_>,input:&SpacesSeparators)->CommandResult<()> {
    match check(cx.doc,input) {Ok(work)=>CommandResult::Completed {output:(),warnings:work.warnings},Err(why)=>why.into()}
}
pub fn execute(cx:&mut ExecutionContext<'_>,input:SpacesSeparators)->CommandResult<SpaceSeparatorsChanged> {
    crate::call::recorded(cx,CAD_SPACES_SEPARATORS,CAD_SPACES_SEPARATORS_VERSION,input,write)
}
fn write(cx:&mut ExecutionContext<'_>,input:SpacesSeparators)->CommandResult<SpaceSeparatorsChanged> {
    let work=match check(cx.doc,&input){Ok(work)=>work,Err(why)=>return why.into()};
    let slots=if let Some(slots)=work.slots {
        cx.doc.update_many(slots.iter().copied().zip(work.entities).collect(),STEP);slots
    } else {
        match cx.doc.add_many(work.entities,STEP) {
            Ok(slots)=>slots,Err(why)=>return Stop::failed(checks::error(codes::SLOTS_EXHAUSTED,why.to_string(),None)).into(),
        }
    };
    CommandResult::Completed {output:SpaceSeparatorsChanged {
        uids:slots.iter().filter_map(|&slot|cx.doc.uid(slot).map(|uid|uid.to_string())).collect(),
        ids:slots.iter().map(|slot|slot.0).collect(),revision:cx.doc.revision().to_string()},warnings:work.warnings}
}
