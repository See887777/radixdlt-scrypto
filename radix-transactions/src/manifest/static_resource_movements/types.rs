use super::*;
use crate::internal_prelude::*;
use indexmap::IndexSet;
use radix_engine_interface::blueprints::account::*;
use radix_engine_interface::prelude::*;

/// A type representing partial knowledge of the balances of some number of resources.
///
/// This type can be used to model the worktop, and can be used for modelling the inbound/
/// outbound resources for any instruction.
///
/// The knowledge is split between specified resources (where we store a [`ResourceBound`]
/// for each resource), and unspecified resources, captured by an [`UnspecifiedResourceKnowledge`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrackedResources {
    /// Captures the bounds of explicitly tracked resources.
    /// Some of these may be
    specified_resources: IndexMap<ResourceAddress, TrackedResource>,
    /// Captures the bounds of unspecified resources.
    unspecified_resources: UnspecifiedResourceKnowledge,
}

impl TrackedResources {
    // Constructors
    pub fn new_empty() -> Self {
        Default::default()
    }

    pub fn new_with_possible_balance_of_unspecified_resources(
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        Self {
            specified_resources: Default::default(),
            unspecified_resources: UnspecifiedResourceKnowledge::SomeBalancesMayBePresent(
                change_sources.into_iter().collect(),
            ),
        }
    }

    // Deconstructors
    pub fn deconstruct(
        self,
    ) -> (
        IndexMap<ResourceAddress, TrackedResource>,
        UnspecifiedResourceKnowledge,
    ) {
        (self.specified_resources, self.unspecified_resources)
    }

    // &self methods
    pub fn specified_resources(&self) -> &IndexMap<ResourceAddress, TrackedResource> {
        &self.specified_resources
    }

    pub fn unspecified_resources(&self) -> &UnspecifiedResourceKnowledge {
        &self.unspecified_resources
    }

    /// Verifies that the bounds are equal, but ignores the sources of those bounds.
    pub fn eq_ignoring_history(&self, other: &Self) -> bool {
        if !self
            .unspecified_resources
            .eq_ignoring_history(&other.unspecified_resources)
        {
            return false;
        }

        // We can't assume self or other are normalized, so it may be that self has a specified resource
        // with a bound equivalent to an unspecified resource bound. Such a resource doesn't need to
        // exist as specified in B.
        // Therefore, instead of just comparing specified_resources, we instead simply check that all
        // bounds of a specified resource in A have the same bound in B (specified or unspecified),
        // AND we check the other way around too.
        for (resource, bound) in self.specified_resources.iter() {
            if !other.resource_status(resource).eq_ignoring_history(bound) {
                return false;
            }
        }
        for (resource, bound) in other.specified_resources.iter() {
            if !self.resource_status(resource).eq_ignoring_history(bound) {
                return false;
            }
        }
        return true;
    }

    /// Works for any resource, specified and unspecified.
    fn resource_status(&self, resource: &ResourceAddress) -> Cow<TrackedResource> {
        match self.specified_resources.get(resource) {
            Some(bound) => Cow::Borrowed(bound),
            None => Cow::Owned(self.unspecified_resources.resource_status()),
        }
    }

    /// Works for any resource, specified and unspecified.
    /// If the resource is unspecified, it makes it specified, then returns a reference to the entry.
    fn resource_status_mut(&mut self, resource: ResourceAddress) -> &mut TrackedResource {
        match self.specified_resources.entry(resource) {
            indexmap::map::Entry::Occupied(occupied_entry) => occupied_entry.into_mut(),
            indexmap::map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(self.unspecified_resources.resource_status())
            }
        }
    }

    /// Removes any specific resources whose bounds are identical to the default.
    ///
    /// We also ensure that any resources that get filtered out have their balance sources
    /// added to the sources for unspecified balances.
    pub fn normalize(self) -> Self {
        let mut unspecified_resources = self.unspecified_resources;
        let mut normalized_bounds: IndexMap<ResourceAddress, TrackedResource> = Default::default();
        let unspecified_resource_bound = unspecified_resources.resource_status();
        for (resource_address, bound) in self.specified_resources {
            if bound.eq_ignoring_history(&unspecified_resource_bound) {
                // We filter out this resource as it's identical
                if !bound.is_zero() {
                    let possible_balance_sources = bound
                        .history
                        .all_additive_change_sources_since_was_last_zero();
                    unspecified_resources
                        .mut_add_possible_resource_balance(possible_balance_sources);
                }
            } else {
                normalized_bounds.insert(resource_address, bound);
            }
        }

        Self {
            specified_resources: normalized_bounds,
            unspecified_resources,
        }
    }

    pub fn add_unspecified_resources(
        mut self,
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        self.mut_add_unspecified_resources(change_sources);
        self
    }

    pub fn mut_add_unspecified_resources(
        &mut self,
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) {
        self.unspecified_resources
            .mut_add(UnspecifiedResourceKnowledge::SomeBalancesMayBePresent(
                change_sources.into_iter().collect(),
            ));
    }

    pub fn add(&mut self, other: TrackedResources) -> Result<(), StaticResourceMovementsError> {
        // For efficiency, we first handle unspecified resources in other.
        // This if statement isn't necessary for correct logic, but offers a small optimization.
        if other.unspecified_resources.may_be_present() {
            for (resource, resource_bound) in &mut self.specified_resources {
                // If an existing resource isn't specified in other, we have to add its unspecified constraints instead
                if !other.specified_resources.contains_key(resource) {
                    resource_bound.add_from(other.unspecified_resources.resource_status())?;
                }
            }
            self.unspecified_resources
                .mut_add(other.unspecified_resources);
        }

        for (other_resource, other_resource_bound) in other.specified_resources {
            self.resource_status_mut(other_resource)
                .add_from(other_resource_bound)?;
        }

        Ok(())
    }

    pub fn add_resource(
        mut self,
        resource: ResourceAddress,
        amount: TrackedResource,
    ) -> Result<Self, StaticResourceMovementsError> {
        self.mut_add_resource(resource, amount)?;
        Ok(self)
    }

    pub fn mut_add_resource(
        &mut self,
        resource: ResourceAddress,
        amount: TrackedResource,
    ) -> Result<(), StaticResourceMovementsError> {
        if resource.is_fungible() && amount.bounds().includes_non_fungible_details() {
            return Err(
                StaticResourceMovementsError::NonFungibleIdsSpecifiedAgainstFungibleResource,
            );
        }
        self.resource_status_mut(resource).add_from(amount)
    }

    pub fn take_resource(
        &mut self,
        resource: ResourceAddress,
        amount: ResourceTakeAmount,
        source: ChangeSource,
    ) -> Result<TrackedResource, StaticResourceMovementsError> {
        self.resource_status_mut(resource).take(amount, source)
    }

    pub fn take_all(&mut self) -> Self {
        core::mem::take(self)
    }

    pub fn handle_worktop_assertion(
        &mut self,
        worktop_assertion: WorktopAssertion,
        source: ChangeSource,
    ) -> Result<(), StaticResourceMovementsError> {
        // FUTURE TWEAK: Could return an optional set of constraints using all_changes
        match worktop_assertion {
            WorktopAssertion::AnyAmountGreaterThanZero { resource_address } => self
                .resource_status_mut(*resource_address)
                .handle_assertion(ResourceBounds::non_zero(), source),
            WorktopAssertion::AtLeastAmount {
                resource_address,
                amount,
            } => self
                .resource_status_mut(*resource_address)
                .handle_assertion(ResourceBounds::at_least_amount(amount)?, source),
            WorktopAssertion::AtLeastNonFungibles {
                resource_address,
                ids,
            } => self
                .resource_status_mut(*resource_address)
                .handle_assertion(
                    ResourceBounds::at_least_non_fungibles(ids.iter().cloned()),
                    source,
                ),
            WorktopAssertion::IsEmpty => {
                for bound in self.specified_resources.values_mut() {
                    // First we handle the assertions to check that it's possible for everything to be zero.
                    bound.handle_assertion(ResourceBounds::zero(), source)?;
                }
                // Then we zero-out all our resource balances and history
                self.specified_resources.clear();
                self.unspecified_resources.clear();
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UnspecifiedResourceKnowledge {
    /// There are no unspecified resources present
    #[default]
    NonePresent,
    /// There might be non-zero balances of unspecified resources present
    SomeBalancesMayBePresent(IndexSet<ChangeSource>),
}

impl UnspecifiedResourceKnowledge {
    pub fn none() -> Self {
        Self::NonePresent
    }

    pub fn clear(&mut self) {
        *self = Self::NonePresent;
    }

    pub fn resource_status(&self) -> TrackedResource {
        match self {
            Self::NonePresent => TrackedResource::zero(),
            Self::SomeBalancesMayBePresent(sources) => {
                TrackedResource::zero_or_more(sources.iter().cloned())
            }
        }
    }

    pub fn resource_bounds(&self) -> ResourceBounds {
        match self {
            Self::NonePresent => ResourceBounds::zero(),
            Self::SomeBalancesMayBePresent(_) => ResourceBounds::zero_or_more(),
        }
    }

    pub fn none_are_present(&self) -> bool {
        match self {
            Self::NonePresent => true,
            Self::SomeBalancesMayBePresent(_) => false,
        }
    }

    pub fn may_be_present(&self) -> bool {
        match self {
            Self::NonePresent => false,
            Self::SomeBalancesMayBePresent(_) => true,
        }
    }

    pub fn add_possible_resource_balance(
        mut self,
        sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        self.mut_add_possible_resource_balance(sources);
        self
    }

    pub fn mut_add_possible_resource_balance(
        &mut self,
        sources: impl IntoIterator<Item = ChangeSource>,
    ) {
        match self {
            mutself @ Self::NonePresent => {
                *mutself = Self::SomeBalancesMayBePresent(sources.into_iter().collect());
            }
            Self::SomeBalancesMayBePresent(self_sources) => {
                self_sources.extend(sources);
            }
        }
    }

    pub fn add(mut self, other: Self) -> Self {
        self.mut_add(other);
        self
    }

    pub fn mut_add(&mut self, other: Self) {
        match other {
            Self::NonePresent => {}
            Self::SomeBalancesMayBePresent(other_sources) => {
                self.mut_add_possible_resource_balance(other_sources);
            }
        }
    }

    /// Verifies that the bounds are equal, but ignores the sources of those bounds.
    pub fn eq_ignoring_history(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NonePresent, Self::NonePresent)
            | (Self::SomeBalancesMayBePresent(_), Self::SomeBalancesMayBePresent(_)) => true,
            (Self::NonePresent, Self::SomeBalancesMayBePresent(_))
            | (Self::SomeBalancesMayBePresent(_), Self::NonePresent) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceTakeAmount {
    Amount(Decimal),
    NonFungibles(IndexSet<NonFungibleLocalId>),
    All,
}

impl ResourceTakeAmount {
    pub fn exact_non_fungibles(ids: impl IntoIterator<Item = NonFungibleLocalId>) -> Self {
        Self::NonFungibles(ids.into_iter().collect())
    }

    pub fn exact_amount(
        amount: impl ResolvableDecimal,
    ) -> Result<Self, StaticResourceMovementsError> {
        let amount = amount.resolve();
        if amount.is_negative() {
            return Err(StaticResourceMovementsError::DecimalAmountIsNegative);
        }
        Ok(Self::Amount(amount))
    }

    pub fn all() -> Self {
        Self::All
    }
}

/// Used to track a known quantity of Fungible and NonFungible resources,
/// for example, the content of a bucket.
///
/// ## Invariants
/// The following inequalities are upheld by all constructors:
/// * `required_ids.len() <= lower_inclusive <= upper_inclusive`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrackedResource {
    /// The current known bounds on the resource balance.
    bounds: ResourceBounds,
    /// This history is only maintained since the last time we knew the balance was zero.
    history: ResourceChangeHistory,
}

impl TrackedResource {
    // Constructors
    pub fn zero() -> Self {
        Self {
            bounds: ResourceBounds::zero(),
            history: ResourceChangeHistory::empty(),
        }
    }

    pub fn exact_amount(
        amount: impl ResolvableDecimal,
        sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Result<Self, StaticResourceMovementsError> {
        Ok(Self::general(
            ResourceBounds::exact_amount(amount)?,
            sources,
        ))
    }

    pub fn at_least_amount(
        amount: impl ResolvableDecimal,
        sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Result<Self, StaticResourceMovementsError> {
        Ok(Self::general(
            ResourceBounds::at_least_amount(amount)?,
            sources,
        ))
    }

    pub fn non_zero(sources: impl IntoIterator<Item = ChangeSource>) -> Self {
        Self::general(ResourceBounds::non_zero(), sources)
    }

    pub fn zero_or_more(sources: impl IntoIterator<Item = ChangeSource>) -> Self {
        Self::general(ResourceBounds::zero_or_more(), sources)
    }

    pub fn non_fungibles(
        ids: impl IntoIterator<Item = NonFungibleLocalId>,
        sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        Self::general(ResourceBounds::exact_non_fungibles(ids), sources)
    }

    pub fn at_least_non_fungibles(
        ids: impl IntoIterator<Item = NonFungibleLocalId>,
        sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        Self::general(ResourceBounds::at_least_non_fungibles(ids), sources)
    }

    pub fn general(
        add_amount: ResourceBounds,
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        Self::new_advanced(
            add_amount.clone(),
            ResourceChangeHistory::empty().record_add(add_amount, change_sources),
        )
    }

    /// This is only pub so that it can be used in tests
    pub fn new_advanced(add_amount: ResourceBounds, history: ResourceChangeHistory) -> Self {
        Self {
            bounds: add_amount,
            history,
        }
    }

    // Deconstructors
    pub fn deconstruct(self) -> (ResourceBounds, ResourceChangeHistory) {
        (self.bounds, self.history)
    }

    // &self methods
    pub fn bounds(&self) -> &ResourceBounds {
        &self.bounds
    }

    /// Returns true if the bound is known to be zero
    pub fn is_zero(&self) -> bool {
        self.bounds.is_zero()
    }

    /// Verifies that the bounds are equal, but ignores the sources of those bounds.
    pub fn eq_ignoring_history(&self, other: &TrackedResource) -> bool {
        self.bounds == other.bounds
    }

    pub fn history(&self) -> &ResourceChangeHistory {
        &self.history
    }

    // &mut self methods

    /// Adds the quantity from the tracked resource, storing its history separately.
    pub fn add_from(
        &mut self,
        existing: TrackedResource,
    ) -> Result<(), StaticResourceMovementsError> {
        self.bounds.mut_add(existing.bounds.clone())?;
        if self.is_zero() {
            self.history.mut_clear();
        } else {
            self.history
                .mut_record_add_with_history(existing.bounds, existing.history);
        }
        Ok(())
    }

    pub fn add(
        &mut self,
        amount: ResourceBounds,
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Result<(), StaticResourceMovementsError> {
        self.bounds.mut_add(amount.clone())?;

        if self.is_zero() {
            self.history.mut_clear();
        } else {
            self.history.mut_record_add(amount, change_sources)
        }

        Ok(())
    }

    pub fn take(
        &mut self,
        take_amount: ResourceTakeAmount,
        source: ChangeSource,
    ) -> Result<TrackedResource, StaticResourceMovementsError> {
        match take_amount {
            ResourceTakeAmount::All => {
                // In the case of a "take all" we just return the existing contents and history,
                // without changing it - and we replace with a blank slate.
                return Ok(core::mem::replace(self, Self::zero()));
            }
            _ => {
                let taken_amount = self.bounds.mut_take(take_amount.clone())?;
                if self.is_zero() {
                    self.history.mut_clear();
                } else {
                    self.history.mut_record_take(take_amount, source);
                }

                // FUTURE TWEAK: Can output an inequality constraint using history.all_changes()
                Ok(Self::general(taken_amount, [source]))
            }
        }
    }

    pub fn take_all(&mut self) -> Self {
        core::mem::replace(self, Self::zero())
    }

    pub fn handle_assertion(
        &mut self,
        assertion: ResourceBounds,
        source: ChangeSource,
    ) -> Result<(), StaticResourceMovementsError> {
        self.bounds.mut_handle_assertion(assertion.clone())?;

        if self.is_zero() {
            self.history.mut_clear();
        } else {
            self.history.mut_record_assertion(assertion, source);
        }

        // FUTURE TWEAK: Can output an inequality constraint using history.all_changes()
        Ok(())
    }
}

/// [`ResourceBounds`] captures constraints on the balance of a single fungible or non-fungible
/// resource.
///
/// It captures four concepts:
///
/// * A set of [`certain_ids`][Self::certain_ids] which are [`NonFungibleLocalId`]s which are
///   required to be in the balance.
/// * A [`lower_inclusive`][Self::lower_inclusive] bound.
/// * An [`upper_inclusive`][Self::upper_inclusive] bound.
/// * Constraints on the [`allowed_ids`][Self::allowed_ids]. These are either [`AllowedIds::Any`]
///   or can be constrained to [`AllowedIds::Allowlist`] of [`NonFungibleLocalId`]s.
///   If this case, the ids in the resource balance must be a subset of the allowlist.
///
/// ## Trait implementations
/// * The [`PartialEq`] / [`Eq`] implementations both are correctly order-independent on the id sets,
///   from the order-independent implementation of [`IndexSet`].
///
/// ## Invariants
///
/// All methods/functions on this class must guarantee that the following invariants are upheld:
///
/// * If `allowed_ids` is [`AllowedIds::Any`]:
///   * `known_ids.len() <= lower_inclusive <= upper_inclusive`
///
/// * If `allowed_ids` is [`AllowedIds::Allowlist(allowlist)`][AllowedIds::Allowlist]:
///   * `known_ids.len() <= lower_inclusive <= upper_inclusive <= allowlist.len()`
///   * `known_ids.is_subset(allowlist)`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceBounds {
    certain_ids: IndexSet<NonFungibleLocalId>,
    lower_bound: LowerBound,
    upper_bound: UpperBound,
    allowed_ids: AllowedIds,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllowedIds {
    Any,
    Allowlist(IndexSet<NonFungibleLocalId>),
}

impl Default for ResourceBounds {
    fn default() -> Self {
        Self::zero()
    }
}

impl ResourceBounds {
    pub fn zero() -> Self {
        Self {
            certain_ids: Default::default(),
            lower_bound: LowerBound::zero(),
            upper_bound: UpperBound::zero(),
            allowed_ids: AllowedIds::Any,
        }
    }

    pub fn zero_or_more() -> Self {
        Self {
            certain_ids: Default::default(),
            lower_bound: LowerBound::zero(),
            upper_bound: UpperBound::unbounded(),
            allowed_ids: AllowedIds::Any,
        }
    }

    pub fn non_zero() -> Self {
        Self {
            certain_ids: Default::default(),
            lower_bound: LowerBound::non_zero(),
            upper_bound: UpperBound::unbounded(),
            allowed_ids: AllowedIds::Any,
        }
    }

    pub fn exact_amount(
        amount: impl ResolvableDecimal,
    ) -> Result<Self, StaticResourceMovementsError> {
        let amount = amount.resolve();
        if amount.is_negative() {
            return Err(StaticResourceMovementsError::DecimalAmountIsNegative);
        }
        Ok(Self {
            certain_ids: Default::default(),
            lower_bound: LowerBound::at_least(amount),
            upper_bound: UpperBound::at_most(amount),
            allowed_ids: AllowedIds::Any,
        })
    }

    pub fn at_least_amount(
        amount: impl ResolvableDecimal,
    ) -> Result<Self, StaticResourceMovementsError> {
        let amount = amount.resolve();
        if amount.is_negative() {
            return Err(StaticResourceMovementsError::DecimalAmountIsNegative);
        }
        Ok(Self {
            certain_ids: Default::default(),
            lower_bound: LowerBound::at_least(amount),
            upper_bound: UpperBound::unbounded(),
            allowed_ids: AllowedIds::Any,
        })
    }

    pub fn exact_non_fungibles(ids: impl IntoIterator<Item = NonFungibleLocalId>) -> Self {
        let ids = ids.into_iter().collect::<IndexSet<_>>();
        let amount_of_ids: Decimal = ids.len().into();
        Self {
            certain_ids: ids.clone(),
            lower_bound: LowerBound::at_least(amount_of_ids),
            upper_bound: UpperBound::at_most(amount_of_ids),
            allowed_ids: AllowedIds::Allowlist(ids),
        }
    }

    pub fn at_least_non_fungibles(
        required_ids: impl IntoIterator<Item = NonFungibleLocalId>,
    ) -> Self {
        let ids = required_ids.into_iter().collect::<IndexSet<_>>();
        let amount_of_ids: Decimal = ids.len().into();
        Self {
            certain_ids: ids,
            lower_bound: LowerBound::at_least(amount_of_ids),
            upper_bound: UpperBound::unbounded(),
            allowed_ids: AllowedIds::Any,
        }
    }

    pub fn general_no_id_allowlist(
        known_ids: impl IntoIterator<Item = NonFungibleLocalId>,
        lower_bound: LowerBound,
        upper_bound: UpperBound,
    ) -> Result<Self, StaticResourceMovementsError> {
        let required_ids = known_ids.into_iter().collect::<IndexSet<_>>();
        let number_of_required_ids = Decimal::from(required_ids.len());
        if number_of_required_ids > lower_bound.equivalent_decimal()
            || lower_bound.equivalent_decimal() > upper_bound.equivalent_decimal()
        {
            return Err(StaticResourceMovementsError::ConstraintBoundsInvalid);
        }
        Ok(Self {
            certain_ids: required_ids,
            lower_bound,
            upper_bound,
            allowed_ids: AllowedIds::Any,
        })
    }

    pub fn general_with_id_allowlist(
        required_ids: impl IntoIterator<Item = NonFungibleLocalId>,
        lower_bound: LowerBound,
        upper_bound: UpperBound,
        id_allowlist: impl IntoIterator<Item = NonFungibleLocalId>,
    ) -> Result<Self, StaticResourceMovementsError> {
        let required_ids = required_ids.into_iter().collect::<IndexSet<_>>();
        let number_of_required_ids = Decimal::from(required_ids.len());
        let id_allowlist = id_allowlist.into_iter().collect::<IndexSet<_>>();
        let number_of_allowed_ids = Decimal::from(id_allowlist.len());

        if number_of_required_ids > lower_bound.equivalent_decimal()
            || lower_bound.equivalent_decimal() > upper_bound.equivalent_decimal()
            || upper_bound.equivalent_decimal() > number_of_allowed_ids
        {
            return Err(StaticResourceMovementsError::ConstraintBoundsInvalid);
        }
        if !required_ids.is_subset(&id_allowlist) {
            return Err(StaticResourceMovementsError::ConstraintBoundsInvalid);
        }
        Ok(Self {
            certain_ids: required_ids,
            lower_bound,
            upper_bound,
            allowed_ids: AllowedIds::Allowlist(id_allowlist),
        })
    }

    pub fn deconstruct(
        self,
    ) -> (
        IndexSet<NonFungibleLocalId>,
        LowerBound,
        UpperBound,
        AllowedIds,
    ) {
        (
            self.certain_ids,
            self.lower_bound,
            self.upper_bound,
            self.allowed_ids,
        )
    }

    // &self methods
    pub fn numeric_bounds(&self) -> (LowerBound, UpperBound) {
        (self.lower_bound, self.upper_bound)
    }

    pub fn certain_ids(&self) -> &IndexSet<NonFungibleLocalId> {
        &self.certain_ids
    }

    pub fn allowed_ids(&self) -> &AllowedIds {
        &self.allowed_ids
    }

    pub fn includes_non_fungible_details(&self) -> bool {
        let is_fungible = self.certain_ids.is_empty() && self.allowed_ids == AllowedIds::Any;
        !is_fungible
    }

    /// Returns true if the bound is known to be zero
    pub fn is_zero(&self) -> bool {
        self.eq(&Self::zero())
    }

    // mut self and &mut self methods
    pub fn add(mut self, other: Self) -> Result<Self, StaticResourceMovementsError> {
        self.mut_add(other)?;
        Ok(self)
    }

    pub fn mut_add(&mut self, other: Self) -> Result<(), StaticResourceMovementsError> {
        self.lower_bound = self.lower_bound.add(other.lower_bound)?;
        self.upper_bound = self.upper_bound.add(other.upper_bound)?;
        for id in other.certain_ids.into_iter() {
            if !self.certain_ids.insert(id) {
                return Err(StaticResourceMovementsError::DuplicateNonFungibleId);
            }
        }

        match (&mut self.allowed_ids, other.allowed_ids) {
            (AllowedIds::Any, _) => {} // If all ids are allowed, keep it that way
            (self_permitted_ids, AllowedIds::Any) => *self_permitted_ids = AllowedIds::Any,
            (AllowedIds::Allowlist(allow_list), AllowedIds::Allowlist(other_allow_list)) => {
                // Unlike the known ids, it's fine for the allow lists to overlap, so don't error on duplicates.
                allow_list.extend(other_allow_list);
            }
        }

        Ok(())
    }

    pub fn take(
        mut self,
        amount: ResourceTakeAmount,
    ) -> Result<Self, StaticResourceMovementsError> {
        self.mut_take(amount)?;
        Ok(self)
    }

    pub fn mut_take(
        &mut self,
        amount: ResourceTakeAmount,
    ) -> Result<Self, StaticResourceMovementsError> {
        match amount {
            ResourceTakeAmount::Amount(take_amount) => {
                if take_amount.is_negative() {
                    return Err(StaticResourceMovementsError::DecimalAmountIsNegative);
                }
                self.upper_bound.take_amount(take_amount)?;
                self.lower_bound.take_amount(take_amount)?;

                // For known ids, we don't know which ids were taken, so we have to clear them.
                // But the allowed ids stay as-is
                if take_amount.is_positive() {
                    self.certain_ids.clear();
                }

                // Taken amount
                Self::exact_amount(take_amount)
            }
            ResourceTakeAmount::NonFungibles(taken_ids) => {
                let take_amount = Decimal::from(taken_ids.len());

                self.lower_bound.take_amount(take_amount)?;
                self.upper_bound.take_amount(take_amount)?;

                // Remove any taken ids from the list of known/required ids.
                // It's okay if some of the taken ids weren't required to be present.
                self.certain_ids = self.certain_ids.difference(&taken_ids).cloned().collect();

                // Finally, we check all the taken ids are in the allow list (if it exists) and these ids
                // are removed from the allow list.
                if let AllowedIds::Allowlist(allow_list) = &mut self.allowed_ids {
                    if !taken_ids.is_subset(allow_list) {
                        return Err(StaticResourceMovementsError::TakeCannotBeSatisfied);
                    }
                    *allow_list = allow_list.difference(&taken_ids).cloned().collect();
                }

                // We check remaining invariants: it's an error if, after taking all matching ids,
                // we now are required to have too many.
                // e.g. This catches "Add A, 1 of 1; Take B, C"
                if Decimal::from(self.certain_ids.len()) > self.lower_bound.equivalent_decimal() {
                    return Err(StaticResourceMovementsError::TakeCannotBeSatisfied);
                }

                // Taken amount
                Ok(Self::exact_non_fungibles(taken_ids))
            }
            ResourceTakeAmount::All => {
                // Taken amount
                Ok(core::mem::replace(self, Self::zero()))
            }
        }
    }

    pub fn handle_assertion(
        mut self,
        assertion: ResourceBounds,
    ) -> Result<Self, StaticResourceMovementsError> {
        self.mut_handle_assertion(assertion)?;
        Ok(self)
    }

    pub fn mut_handle_assertion(
        &mut self,
        assertion: ResourceBounds,
    ) -> Result<(), StaticResourceMovementsError> {
        // Possibly increase lower bound and decrease upper bound
        self.lower_bound = self.lower_bound.handle_assertion(assertion.lower_bound);
        self.upper_bound = self.upper_bound.handle_assertion(assertion.upper_bound);

        // Handle the allow list
        if let AllowedIds::Allowlist(assertion_allowlist) = assertion.allowed_ids {
            // Check the known/required ids are a subset of the assertion allowlist
            if !self.certain_ids.is_subset(&assertion_allowlist) {
                return Err(StaticResourceMovementsError::AssertionCannotBeSatisfied);
            }
            // Intersect the allow lists
            match &mut self.allowed_ids {
                allowed_ids @ AllowedIds::Any => {
                    *allowed_ids = AllowedIds::Allowlist(assertion_allowlist);
                }
                AllowedIds::Allowlist(allowlist) => {
                    *allowlist = allowlist
                        .intersection(&assertion_allowlist)
                        .cloned()
                        .collect();
                }
            }
        }

        // We've already checked that our certain ids are in the assertion allowlist
        // (and therefore, using the invariant, are in the intersection).
        // We now need to complete processing by expanding the known ids list according to the assertion.
        for required_id in assertion.certain_ids.iter() {
            self.certain_ids.insert(required_id.clone());
        }

        // Finally, we verify the invariants are still upheld.
        // At this point, assuming self and the assertion satisfied the original invariants,
        // given the work we've done above, we know that:
        // * self.required_ids.len() <= self.lower_inclusive
        // * self.required_ids.is_subset(self.allowlist)
        //
        // We still need to check two more which could now have been invalidated:
        // * self.lower_inclusive <= self.upper_inclusive
        // * self.upper_inclusive <= self.allowlist.len()
        if self.lower_bound.equivalent_decimal() > self.upper_bound.equivalent_decimal() {
            return Err(StaticResourceMovementsError::AssertionCannotBeSatisfied);
        }

        if let AllowedIds::Allowlist(allowlist) = &self.allowed_ids {
            if self.upper_bound.equivalent_decimal() > Decimal::from(allowlist.len()) {
                return Err(StaticResourceMovementsError::AssertionCannotBeSatisfied);
            }
        }

        Ok(())
    }

    /// For situations where someone has taken an unknown amount from the balance.
    pub fn replace_lower_bounds_with_zero(mut self) -> Self {
        self.mut_replace_lower_bounds_with_zero();
        self
    }

    /// For situations where someone has taken an unknown amount from the balance.
    pub fn mut_replace_lower_bounds_with_zero(&mut self) {
        self.certain_ids = Default::default();
        self.lower_bound = LowerBound::zero();
    }
}

/// ## Invariants
/// * The `amount` in `LowerBound::AmountInclusive(amount) is required to be non-negative.
///
/// ## Trait Implementations
/// * [`Ord`], [`PartialOrd`] - Satisfies `AmountInclusive(Zero) < NonZero < AmountInclusive(AnyPositive)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowerBound {
    AmountInclusive(Decimal),
    NonZero,
}

impl PartialOrd for LowerBound {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LowerBound {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self, other) {
            (
                LowerBound::AmountInclusive(self_lower_inclusive),
                LowerBound::AmountInclusive(other_lower_inclusive),
            ) => self_lower_inclusive.cmp(other_lower_inclusive),
            (LowerBound::AmountInclusive(self_lower_inclusive), LowerBound::NonZero) => {
                if self_lower_inclusive.is_positive() {
                    core::cmp::Ordering::Greater
                } else {
                    core::cmp::Ordering::Less
                }
            }
            (LowerBound::NonZero, LowerBound::AmountInclusive(other_lower_inclusive)) => {
                if other_lower_inclusive.is_positive() {
                    core::cmp::Ordering::Less
                } else {
                    core::cmp::Ordering::Greater
                }
            }
            (LowerBound::NonZero, LowerBound::NonZero) => core::cmp::Ordering::Equal,
        }
    }
}

impl LowerBound {
    pub const fn zero() -> Self {
        Self::AmountInclusive(Decimal::ZERO)
    }

    pub const fn non_zero() -> Self {
        Self::NonZero
    }

    pub fn cmp_upper(&self, other: &UpperBound) -> core::cmp::Ordering {
        match (self, other) {
            (
                LowerBound::AmountInclusive(lower_bound_inclusive),
                UpperBound::AmountInclusive(upper_bound_inclusive),
            ) => lower_bound_inclusive.cmp(upper_bound_inclusive),
            (_, UpperBound::Unbounded) => core::cmp::Ordering::Less,
            (LowerBound::NonZero, UpperBound::AmountInclusive(upper_bound_inclusive)) => {
                if upper_bound_inclusive.is_zero() {
                    core::cmp::Ordering::Greater
                } else {
                    core::cmp::Ordering::Less
                }
            }
        }
    }

    /// ## Panics
    /// * Panics if the decimal is not resolvable or is non-negative
    pub fn at_least(decimal: impl ResolvableDecimal) -> Self {
        let decimal = decimal.resolve();
        if decimal.is_negative() {
            panic!("An at_least bound is negative");
        }
        Self::AmountInclusive(decimal)
    }

    pub fn is_zero(&self) -> bool {
        self.eq(&Self::zero())
    }

    pub fn is_positive(&self) -> bool {
        !self.is_zero()
    }

    fn add(self, other: Self) -> Result<Self, StaticResourceMovementsError> {
        let bound = match (self, other) {
            (
                LowerBound::AmountInclusive(self_lower_bound),
                LowerBound::AmountInclusive(other_lower_bound),
            ) => {
                let lower_bound_inclusive = self_lower_bound
                    .checked_add(other_lower_bound)
                    .ok_or(StaticResourceMovementsError::DecimalOverflow)?;
                LowerBound::AmountInclusive(lower_bound_inclusive)
            }
            (LowerBound::AmountInclusive(amount), LowerBound::NonZero)
            | (LowerBound::NonZero, LowerBound::AmountInclusive(amount)) => {
                if amount.is_zero() {
                    LowerBound::NonZero
                } else {
                    LowerBound::AmountInclusive(amount)
                }
            }
            (LowerBound::NonZero, LowerBound::NonZero) => LowerBound::NonZero,
        };

        Ok(bound)
    }

    /// PRECONDITION: take_amount must be positive
    fn take_amount(self, take_amount: Decimal) -> Result<Self, StaticResourceMovementsError> {
        let bound = match self {
            LowerBound::AmountInclusive(lower_bound_inclusive) => {
                if take_amount > lower_bound_inclusive {
                    Self::zero()
                } else {
                    LowerBound::AmountInclusive(lower_bound_inclusive - take_amount)
                }
            }
            LowerBound::NonZero => {
                if take_amount.is_zero() {
                    LowerBound::NonZero
                } else {
                    Self::zero()
                }
            }
        };

        Ok(bound)
    }

    fn handle_assertion(self, assertion_bound: LowerBound) -> Self {
        self.max(assertion_bound)
    }

    pub fn equivalent_decimal(&self) -> Decimal {
        match self {
            LowerBound::AmountInclusive(decimal) => *decimal,
            LowerBound::NonZero => Decimal::from_attos(I192::ONE),
        }
    }
}

/// ## Trait Implementations
/// * [`Ord`], [`PartialOrd`] - Satisfies `AmountInclusive(Any) < Unbounded`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpperBound {
    AmountInclusive(Decimal),
    Unbounded,
}

impl PartialOrd for UpperBound {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UpperBound {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self, other) {
            (
                UpperBound::AmountInclusive(upper_bound_inclusive),
                UpperBound::AmountInclusive(other_upper_bound_inclusive),
            ) => upper_bound_inclusive.cmp(other_upper_bound_inclusive),
            (UpperBound::AmountInclusive(_), UpperBound::Unbounded) => core::cmp::Ordering::Less,
            (UpperBound::Unbounded, UpperBound::AmountInclusive(_)) => core::cmp::Ordering::Greater,
            (UpperBound::Unbounded, UpperBound::Unbounded) => core::cmp::Ordering::Equal,
        }
    }
}

impl UpperBound {
    pub const fn unbounded() -> Self {
        Self::Unbounded
    }

    pub const fn zero() -> Self {
        Self::AmountInclusive(Decimal::ZERO)
    }

    /// ## Panics
    /// * Panics if the decimal is not resolvable or is non-negative
    pub fn at_most(decimal: impl ResolvableDecimal) -> Self {
        let decimal = decimal.resolve();
        if decimal.is_negative() {
            panic!("An at_most bound is negative");
        }
        Self::AmountInclusive(decimal.resolve())
    }

    fn add(self, other: Self) -> Result<Self, StaticResourceMovementsError> {
        let bound = match (self, other) {
            (
                UpperBound::AmountInclusive(self_upper_bound_inclusive),
                UpperBound::AmountInclusive(other_upper_bound_inclusive),
            ) => {
                let upper_bound_inclusive = self_upper_bound_inclusive
                    .checked_add(other_upper_bound_inclusive)
                    .ok_or(StaticResourceMovementsError::DecimalOverflow)?;
                UpperBound::AmountInclusive(upper_bound_inclusive)
            }
            (_, UpperBound::Unbounded) | (UpperBound::Unbounded, _) => UpperBound::Unbounded,
        };

        Ok(bound)
    }

    /// PRECONDITION: take_amount must be positive
    fn take_amount(self, take_amount: Decimal) -> Result<Self, StaticResourceMovementsError> {
        let bound = match self {
            UpperBound::AmountInclusive(upper_bound_inclusive) => {
                if take_amount > upper_bound_inclusive {
                    return Err(StaticResourceMovementsError::TakeCannotBeSatisfied);
                }
                UpperBound::AmountInclusive(upper_bound_inclusive - take_amount)
            }
            UpperBound::Unbounded => UpperBound::Unbounded,
        };

        Ok(bound)
    }

    fn handle_assertion(self, assertion_bound: UpperBound) -> Self {
        self.min(assertion_bound)
    }

    pub fn equivalent_decimal(&self) -> Decimal {
        match self {
            UpperBound::AmountInclusive(decimal) => *decimal,
            UpperBound::Unbounded => Decimal::MAX,
        }
    }
}

/// Intended to save all history since the balance was known to be zero.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResourceChangeHistory(Vec<ResourceChange>);

impl ResourceChangeHistory {
    pub fn empty() -> Self {
        Self(vec![])
    }

    pub fn mut_clear(&mut self) {
        self.0.clear();
    }

    pub fn mut_record(&mut self, change: ResourceChange) {
        self.0.push(change);
    }

    pub fn record_take(
        mut self,
        take_amount: ResourceTakeAmount,
        change_source: ChangeSource,
    ) -> Self {
        self.mut_record_take(take_amount, change_source);
        self
    }

    pub fn mut_record_take(
        &mut self,
        take_amount: ResourceTakeAmount,
        change_source: ChangeSource,
    ) {
        self.0.push(ResourceChange::Take {
            take_amount,
            change_source,
        });
    }

    pub fn record_add(
        mut self,
        add_amount: ResourceBounds,
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) -> Self {
        self.mut_record_add(add_amount, change_sources);
        self
    }

    pub fn mut_record_add(
        &mut self,
        add_amount: ResourceBounds,
        change_sources: impl IntoIterator<Item = ChangeSource>,
    ) {
        self.0.push(ResourceChange::Add {
            add_amount,
            change_sources: change_sources.into_iter().collect(),
        });
    }

    /// It only records a forked `AddWithOwnHistory` if the timeline is non-trivial (i.e. not just a single add).
    /// We try our best to avoid forks, and only fork if we have to
    pub fn mut_record_add_with_history(
        &mut self,
        add_amount: ResourceBounds,
        change_history: ResourceChangeHistory,
    ) {
        match change_history.0.len() {
            0 => {
                // Only exists if add_amount is 0
            }
            1 => {
                let Ok([single_history_item]) = <[ResourceChange; 1]>::try_from(change_history.0)
                else {
                    unreachable!()
                };
                // Only exists if add_amount is Add or AddWithOwnHistory
                self.mut_record(single_history_item);
            }
            _ => {
                if self.0.len() == 0 {
                    *self = change_history
                } else {
                    self.0.push(ResourceChange::AddWithForkedHistory {
                        add_amount,
                        change_history,
                    });
                }
            }
        }
    }

    pub fn record_assertion(
        mut self,
        assertion: ResourceBounds,
        change_source: ChangeSource,
    ) -> Self {
        self.mut_record_assertion(assertion, change_source);
        self
    }

    pub fn mut_record_assertion(&mut self, assertion: ResourceBounds, change_source: ChangeSource) {
        self.0.push(ResourceChange::Assertion {
            assertion,
            change_source,
        })
    }

    pub fn all_changes(&self) -> impl Iterator<Item = &ResourceChange> {
        self.0.iter()
    }

    pub fn all_additive_change_sources_since_was_last_zero(&self) -> IndexSet<ChangeSource> {
        // This could be done more efficiently if we cache the partial totals at each stage.
        let mut cumulative = ResourceBounds::zero();
        let mut all_change_sources: IndexSet<ChangeSource> = Default::default();
        for resource_change in self.all_changes() {
            match resource_change {
                ResourceChange::Add {
                    add_amount,
                    change_sources,
                } => {
                    cumulative.mut_add(add_amount.clone()).unwrap();
                    all_change_sources.extend(change_sources);
                }
                ResourceChange::AddWithForkedHistory {
                    add_amount,
                    change_history,
                } => {
                    cumulative.mut_add(add_amount.clone()).unwrap();
                    all_change_sources
                        .extend(change_history.all_additive_change_sources_since_was_last_zero());
                }
                ResourceChange::Take { take_amount, .. } => {
                    cumulative.mut_take(take_amount.clone()).unwrap();
                }
                ResourceChange::Assertion { assertion, .. } => {
                    cumulative.mut_handle_assertion(assertion.clone()).unwrap();
                }
            }
            if cumulative.is_zero() {
                all_change_sources.clear();
            }
        }
        all_change_sources
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceChange {
    Add {
        add_amount: ResourceBounds,
        change_sources: IndexSet<ChangeSource>,
    },
    AddWithForkedHistory {
        add_amount: ResourceBounds,
        change_history: ResourceChangeHistory,
    },
    Take {
        take_amount: ResourceTakeAmount,
        change_source: ChangeSource,
    },
    Assertion {
        assertion: ResourceBounds,
        change_source: ChangeSource,
    },
}

//====================================================

#[derive(Debug, Clone)]
pub struct StaticResourceMovementsOutput {
    pub invocation_static_information: IndexMap<usize, InvocationStaticInformation>,
}

impl StaticResourceMovementsOutput {
    pub fn account_withdraws(&self) -> IndexMap<ComponentAddress, Vec<AccountWithdraw>> {
        let mut withdrawals: IndexMap<ComponentAddress, Vec<AccountWithdraw>> = Default::default();

        for invocation in self.invocation_static_information.values() {
            let Some((account_address, method)) = invocation.as_account_method() else {
                continue;
            };
            let is_fungible_withdraw = matches!(
                method,
                ACCOUNT_WITHDRAW_IDENT | ACCOUNT_LOCK_FEE_AND_WITHDRAW_IDENT
            );
            let is_non_fungible_withdraw = matches!(
                method,
                ACCOUNT_WITHDRAW_NON_FUNGIBLES_IDENT
                    | ACCOUNT_LOCK_FEE_AND_WITHDRAW_NON_FUNGIBLES_IDENT
            );
            if !(is_fungible_withdraw || is_non_fungible_withdraw) {
                continue;
            }
            let account_withdrawal = {
                if invocation.output.unspecified_resources().may_be_present() {
                    panic!("Account withdraw output should not have unspecified resources");
                }
                let resources = invocation.output.specified_resources();
                if resources.len() != 1 {
                    panic!("Account withdraw output should have exactly one resource");
                }
                let (resource_address, specified_resource) = resources.first().unwrap();
                if is_non_fungible_withdraw {
                    AccountWithdraw::Ids(
                        *resource_address,
                        specified_resource.bounds().certain_ids().clone(),
                    )
                } else {
                    AccountWithdraw::Amount(
                        *resource_address,
                        specified_resource.bounds.lower_bound.equivalent_decimal(),
                    )
                }
            };
            withdrawals
                .entry(account_address)
                .or_default()
                .push(account_withdrawal);
        }

        withdrawals
    }

    pub fn account_deposits(&self) -> IndexMap<ComponentAddress, Vec<AccountDeposit>> {
        let mut deposits: IndexMap<ComponentAddress, Vec<AccountDeposit>> = Default::default();

        for invocation in self.invocation_static_information.values() {
            let Some((account_address, method)) = invocation.as_account_method() else {
                continue;
            };

            let is_deposit = matches!(
                method,
                ACCOUNT_DEPOSIT_IDENT
                    | ACCOUNT_DEPOSIT_BATCH_IDENT
                    | ACCOUNT_TRY_DEPOSIT_OR_ABORT_IDENT
                    | ACCOUNT_TRY_DEPOSIT_BATCH_OR_ABORT_IDENT
                    | ACCOUNT_TRY_DEPOSIT_OR_REFUND_IDENT
                    | ACCOUNT_TRY_DEPOSIT_BATCH_OR_REFUND_IDENT
            );

            if !is_deposit {
                continue;
            }

            let (specified_resources, unspecified_resources) =
                invocation.input.clone().normalize().deconstruct();
            let mut account_deposit = AccountDeposit::empty(unspecified_resources);
            for (resource_address, tracked_resource) in specified_resources {
                let (bounds, _history) = tracked_resource.deconstruct();
                account_deposit = account_deposit.set(resource_address, bounds);
            }

            deposits
                .entry(account_address)
                .or_default()
                .push(account_deposit);
        }

        deposits
    }
}

#[derive(Clone, Debug)]
pub struct InvocationStaticInformation {
    pub kind: OwnedInvocationKind,
    pub input: TrackedResources,
    pub output: TrackedResources,
}

impl InvocationStaticInformation {
    pub fn as_account_method(&self) -> Option<(ComponentAddress, &str)> {
        let InvocationStaticInformation {
            kind:
                OwnedInvocationKind::Method {
                    address: DynamicGlobalAddress::Static(global_address),
                    module_id: ModuleId::Main,
                    method,
                },
            ..
        } = self
        else {
            return None;
        };
        let Ok(component_address) = ComponentAddress::try_from(*global_address) else {
            return None;
        };
        if !component_address.as_node_id().is_global_account() {
            return None;
        }
        Some((component_address, method.as_str()))
    }
}

#[derive(Clone, Debug)]
pub enum OwnedInvocationKind {
    Method {
        address: DynamicGlobalAddress,
        module_id: ModuleId,
        method: String,
    },
    Function {
        address: DynamicPackageAddress,
        blueprint: String,
        function: String,
    },
    DirectMethod {
        address: InternalAddress,
        method: String,
    },
    YieldToParent,
    YieldToChild {
        child_index: ManifestNamedIntent,
    },
}

impl<'a> From<InvocationKind<'a>> for OwnedInvocationKind {
    fn from(value: InvocationKind<'a>) -> Self {
        match value {
            InvocationKind::Method {
                address,
                module_id,
                method,
            } => Self::Method {
                address: *address,
                module_id,
                method: method.to_owned(),
            },
            InvocationKind::Function {
                address,
                blueprint,
                function,
            } => Self::Function {
                address: *address,
                blueprint: blueprint.to_owned(),
                function: function.to_owned(),
            },
            InvocationKind::DirectMethod { address, method } => Self::DirectMethod {
                address: *address,
                method: method.to_owned(),
            },
            InvocationKind::YieldToParent => Self::YieldToParent,
            InvocationKind::YieldToChild { child_index } => Self::YieldToChild { child_index },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AccountWithdraw {
    Amount(ResourceAddress, Decimal),
    Ids(ResourceAddress, IndexSet<NonFungibleLocalId>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccountDeposit {
    specified_resources: IndexMap<ResourceAddress, SimpleResourceBounds>,
    unspecified_resources: UnspecifiedResourceKnowledge,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimpleResourceBounds {
    ExactAmount(Decimal),
    AmountRange(LowerBound, UpperBound),
    ExactNonFungibles(IndexSet<NonFungibleLocalId>),
    GeneralNonFungibleBounds(ResourceBounds),
}

impl From<SimpleResourceBounds> for ResourceBounds {
    fn from(value: SimpleResourceBounds) -> Self {
        match value {
            SimpleResourceBounds::ExactAmount(amount) => {
                ResourceBounds::exact_amount(amount).unwrap()
            }
            SimpleResourceBounds::AmountRange(lower_bound, upper_bound) => {
                ResourceBounds::general_no_id_allowlist([], lower_bound, upper_bound).unwrap()
            }
            SimpleResourceBounds::ExactNonFungibles(ids) => {
                ResourceBounds::exact_non_fungibles(ids)
            }
            SimpleResourceBounds::GeneralNonFungibleBounds(resource_bounds) => resource_bounds,
        }
    }
}

impl From<ResourceBounds> for SimpleResourceBounds {
    fn from(value: ResourceBounds) -> Self {
        if value.includes_non_fungible_details() {
            match value.allowed_ids() {
                // Note - IndexSet equality does a set equality, ignoring order
                AllowedIds::Allowlist(allowlist) if value.certain_ids() == allowlist => {
                    let (certain_ids, _, _, _) = value.deconstruct();
                    SimpleResourceBounds::ExactNonFungibles(certain_ids)
                }
                AllowedIds::Any | AllowedIds::Allowlist(_) => {
                    SimpleResourceBounds::GeneralNonFungibleBounds(value)
                }
            }
        } else {
            let (lower_bound, upper_bound) = value.numeric_bounds();
            if lower_bound.cmp_upper(&upper_bound).is_eq() {
                SimpleResourceBounds::ExactAmount(lower_bound.equivalent_decimal())
            } else {
                SimpleResourceBounds::AmountRange(lower_bound, upper_bound)
            }
        }
    }
}

impl SimpleResourceBounds {
    pub fn to_bounds(self) -> SimpleResourceBounds {
        self.into()
    }
}

impl AccountDeposit {
    pub fn empty(unspecified_resources: UnspecifiedResourceKnowledge) -> Self {
        Self {
            specified_resources: Default::default(),
            unspecified_resources,
        }
    }

    /// Should only be used if it doesn't already exist
    pub fn set(mut self, resource_address: ResourceAddress, bounds: ResourceBounds) -> Self {
        self.specified_resources
            .insert(resource_address, bounds.into());
        self
    }

    pub fn unspecified_resources(&self) -> UnspecifiedResourceKnowledge {
        self.unspecified_resources.clone()
    }

    pub fn bounds_for(&self, resource_address: ResourceAddress) -> ResourceBounds {
        match self.specified_resources.get(&resource_address) {
            Some(bounds) => bounds.clone().into(),
            None => self.unspecified_resources.resource_bounds(),
        }
    }
}
