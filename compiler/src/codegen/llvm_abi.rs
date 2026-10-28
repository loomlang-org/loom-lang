//! Callers provide one private stack snapshot for each large value parameter.

use super::*;
use inkwell::values::BasicMetadataValueEnum;

pub(super) fn indirect(native: BasicTypeEnum<'_>, target: &TargetData) -> bool {
    matches!(native, BasicTypeEnum::StructType(_))
        && target.get_abi_size(&native) > 4 * u64::from(target.get_pointer_byte_size(None))
}

pub(super) fn parameter<'ctx>(
    context: &'ctx Context,
    program: &checked::Program,
    target: &TargetData,
    ty: Type,
) -> NativeResult<BasicTypeEnum<'ctx>> {
    let native = native_type(context, program, ty)?;
    Ok(if indirect(native, target) {
        context.ptr_type(AddressSpace::default()).into()
    } else {
        native
    })
}

impl<'ctx> FunctionEmitter<'_, 'ctx> {
    pub(super) fn abi_arguments(
        &mut self,
        params: &[Type],
        values: &[BasicValueEnum<'ctx>],
    ) -> NativeResult<Vec<BasicMetadataValueEnum<'ctx>>> {
        let mut arguments = Vec::with_capacity(values.len());
        let mut used = HashMap::<Type, usize>::new();
        for (ty, value) in params.iter().zip(values) {
            let native = native_type(self.context, self.program, *ty)?;
            arguments.push(if indirect(native, self.target_data) {
                // All source operands have already run and their managed
                // snapshots have been reloaded. This private copy belongs to
                // the invocation; the callee roots it before any safe point.
                // No source value can retain its stack address after the call.
                // Nested operand calls are finished, so sequential invocations
                // can reuse storage. Only arguments in this call overlap.
                let ordinal = used.entry(*ty).or_default();
                let slot = if let Some(slot) = self
                    .argument_slots
                    .get(ty)
                    .and_then(|slots| slots.get(*ordinal))
                {
                    *slot
                } else {
                    let entry = self.context.create_builder();
                    let block = self
                        .function
                        .get_first_basic_block()
                        .ok_or("missing entry")?;
                    if let Some(first) = block.get_first_instruction() {
                        entry.position_before(&first);
                    } else {
                        entry.position_at_end(block);
                    }
                    let slot = entry.build_alloca(native, "argument.value")?;
                    self.argument_slots.entry(*ty).or_default().push(slot);
                    slot
                };
                *ordinal += 1;
                self.builder.build_store(slot, *value)?;
                slot.into()
            } else {
                (*value).into()
            });
        }
        Ok(arguments)
    }
}
