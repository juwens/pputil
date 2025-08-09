use std::rc::Rc;

use crate::helpers::ProvisioningProfileFileData;

pub type ProfilesCollection = Vec<Result<Rc<ProvisioningProfileFileData>, Rc<ProvisioningProfileFileData>>>;
pub type ProfilesCollectionPure = Vec<Rc<ProvisioningProfileFileData>>;