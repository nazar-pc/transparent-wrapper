use transparent_wrapper::TransparentWrapper;

pub(crate) const fn generic_peel<W>(wrapper: &W) -> &W::Inner
where
    W: [const] TransparentWrapper,
{
    wrapper.peel_ref()
}
