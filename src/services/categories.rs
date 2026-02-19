use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;
use pushkind_common::zmq::ZmqSenderExt;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::types::HubId;
use crate::domain::zmq::ZMQCrawlerMessage;
use crate::dto::categories::CategoryDto;
use crate::forms::categories::{
    AddCategoryFormPayload, ClearProductCategoryFormPayload, DeleteCategoryFormPayload,
    SetProductCategoryFormPayload, UpdateCategoryFormPayload,
};
use crate::repository::{
    CategoryListQuery, CategoryReader, CategoryWriter, CrawlerReader, ProductReader, ProductWriter,
};

use super::{ServiceError, ServiceResult};

pub fn show_categories<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<Vec<CategoryDto>>
where
    R: CategoryReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    match repo.list_categories(CategoryListQuery::new(hub_id)) {
        Ok((_total, categories)) => Ok(categories.into_iter().map(CategoryDto::from).collect()),
        Err(e) => {
            log::error!("Failed to list categories: {e}");
            Err(ServiceError::Internal)
        }
    }
}

pub fn add_category<R>(
    payload: AddCategoryFormPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: CategoryWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    let category = payload.into_new_category(hub_id);
    match repo.create_category(&category) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to create category: {e}");
            Ok(false)
        }
    }
}

pub fn update_category<R>(
    payload: UpdateCategoryFormPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: CategoryReader + CategoryWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    match repo.get_category_by_id(payload.category_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get category: {e}");
            return Err(ServiceError::Internal);
        }
    }

    match repo.update_category(
        payload.category_id,
        hub_id,
        &payload.name,
        payload.embedding.as_deref(),
    ) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to update category: {e}");
            Ok(false)
        }
    }
}

pub fn delete_category<R>(
    payload: DeleteCategoryFormPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: CategoryReader + CategoryWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    match repo.get_category_by_id(payload.category_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get category: {e}");
            return Err(ServiceError::Internal);
        }
    }

    match repo.delete_category(payload.category_id, hub_id) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to delete category: {e}");
            Ok(false)
        }
    }
}

pub fn set_product_category_manual<R>(
    payload: SetProductCategoryFormPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: ProductReader + ProductWriter + CrawlerReader + CategoryReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    let product = match repo.get_product_by_id(payload.product_id) {
        Ok(Some(product)) => product,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get product: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.get_crawler_by_id(product.crawler_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return Err(ServiceError::Internal);
        }
    }

    match repo.get_category_by_id(payload.category_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get category by id: {e}");
            return Err(ServiceError::Internal);
        }
    }

    match repo.set_product_category_manual(product.id, payload.category_id) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to set manual category assignment: {e}");
            Ok(false)
        }
    }
}

pub fn clear_product_category_manual<R>(
    payload: ClearProductCategoryFormPayload,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: ProductReader + ProductWriter + CrawlerReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    let product = match repo.get_product_by_id(payload.product_id) {
        Ok(Some(product)) => product,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get product: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.get_crawler_by_id(product.crawler_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return Err(ServiceError::Internal);
        }
    }

    match repo.clear_product_category_manual(product.id) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to clear manual category assignment: {e}");
            Ok(false)
        }
    }
}

pub async fn match_product_categories<S>(
    user: &AuthenticatedUser,
    sender: &S,
) -> ServiceResult<bool>
where
    S: ZmqSenderExt + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    let message = ZMQCrawlerMessage::ProductCategoryMatch(hub_id);
    match sender.send_json(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            log::error!("Failed to send ZMQ message");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::category::Category;
    use crate::domain::crawler::Crawler;
    use crate::domain::product::Product;
    use crate::domain::types::{
        CategoryAssignmentSource, CategoryId, CategoryName, CrawlerId, CrawlerName,
        CrawlerSelectorValue, CrawlerUrl, HubId, ProductCount, ProductId, ProductName,
        ProductPrice, ProductSku, ProductUrl,
    };
    use crate::forms::categories::SetProductCategoryFormPayload;
    use crate::repository::test::TestRepository;
    use chrono::DateTime;

    fn sample_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".into(),
            email: "test@example.com".into(),
            hub_id: 1,
            name: "Test".into(),
            roles: vec![SERVICE_ACCESS_ROLE.into()],
            exp: 0,
        }
    }

    fn sample_crawler() -> Crawler {
        Crawler {
            id: CrawlerId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CrawlerName::new("crawler").unwrap(),
            url: CrawlerUrl::new("http://example.com").unwrap(),
            selector: CrawlerSelectorValue::new("selector").unwrap(),
            processing: false,
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            num_products: ProductCount::new(0).unwrap(),
        }
    }

    fn sample_product() -> Product {
        Product {
            id: ProductId::new(1).unwrap(),
            crawler_id: CrawlerId::new(1).unwrap(),
            name: ProductName::new("Product").unwrap(),
            sku: ProductSku::new("SKU").unwrap(),
            category: None,
            units: None,
            price: ProductPrice::new(1.0).unwrap(),
            amount: None,
            description: None,
            url: ProductUrl::new("http://example.com/p").unwrap(),
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            embedding: None,
            category_id: None,
            category_assignment_source: CategoryAssignmentSource::Automatic,
            images: vec![],
        }
    }

    fn sample_category() -> Category {
        Category {
            id: CategoryId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CategoryName::new("Tea/Green").unwrap(),
            embedding: Some(vec![1, 2, 3]),
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
        }
    }

    #[test]
    fn shows_categories_for_authorized_user() {
        let repo =
            TestRepository::new(vec![], vec![], vec![]).with_categories(vec![sample_category()]);
        let user = sample_user();

        let categories = show_categories(&user, &repo).unwrap();
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].id, 1);
    }

    #[test]
    fn manual_set_requires_existing_category_in_hub() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![sample_product()], vec![]);
        let user = sample_user();
        let payload = SetProductCategoryFormPayload {
            product_id: ProductId::new(1).unwrap(),
            category_id: CategoryId::new(999).unwrap(),
        };

        let err = set_product_category_manual(payload, &user, &repo).unwrap_err();
        assert!(matches!(err, ServiceError::NotFound));
    }

    #[test]
    fn manual_set_succeeds_when_entities_are_hub_scoped() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![sample_product()], vec![])
            .with_categories(vec![sample_category()]);
        let user = sample_user();
        let payload = SetProductCategoryFormPayload {
            product_id: ProductId::new(1).unwrap(),
            category_id: CategoryId::new(1).unwrap(),
        };

        assert!(set_product_category_manual(payload, &user, &repo).unwrap());
    }
}
