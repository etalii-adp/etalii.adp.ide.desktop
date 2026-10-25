//! The surface a designer of the `hierarchy` kind lays out (DESL section 7.7, etalii.adp spec 036): the [`model`]
//! that crosses to the shell, the standalone host's `HierarchySurfaceModel` at a7fdf03, the [`session`] a connection
//! holds, its `IHierarchySurfaceSession`, and the [`definition`] a designer declares for the page to draw it, its
//! `hierarchySurfaceDefinition.ts`. None knows a designer type: the operations, roles, labels and kinds are names a
//! module reads from its definition.

pub mod definition;
pub mod model;
pub mod session;

#[cfg(test)]
mod tests;
