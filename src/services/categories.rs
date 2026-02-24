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
    CategoryListQuery, CategoryReader, CategoryWriter, CrawlerReader, ProcessingStateReader,
    ProductReader, ProductWriter,
};

use super::{ServiceError, ServiceResult};

const CATEGORY_MATCH_PROCESSING_MESSAGE: &str =
    "Матчинг категорий недоступен: дождитесь завершения активной обработки парсеров и бенчмарков.";

fn category_match_available_in_hub<R>(repo: &R, hub_id: HubId) -> ServiceResult<bool>
where
    R: ProcessingStateReader,
{
    match repo.has_active_processing(hub_id) {
        Ok(has_active_processing) => Ok(!has_active_processing),
        Err(e) => {
            log::error!("Failed to read processing state: {e}");
            Err(ServiceError::Internal)
        }
    }
}

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

pub fn can_match_product_categories<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<bool>
where
    R: ProcessingStateReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    category_match_available_in_hub(repo, hub_id)
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

pub async fn match_product_categories<R, S>(
    user: &AuthenticatedUser,
    repo: &R,
    sender: &S,
) -> ServiceResult<bool>
where
    R: ProcessingStateReader,
    S: ZmqSenderExt + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|e| {
        log::error!("Invalid hub id in user context: {e}");
        ServiceError::Internal
    })?;

    if !category_match_available_in_hub(repo, hub_id)? {
        return Err(ServiceError::Form(
            CATEGORY_MATCH_PROCESSING_MESSAGE.to_string(),
        ));
    }

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
    use crate::domain::benchmark::Benchmark;
    use crate::domain::category::Category;
    use crate::domain::crawler::Crawler;
    use crate::domain::product::Product;
    use crate::domain::types::{
        BenchmarkId, BenchmarkName, BenchmarkSku, CategoryAssignmentSource, CategoryId,
        CategoryName, CrawlerId, CrawlerName, CrawlerSelectorValue, CrawlerUrl, HubId,
        ProductAmount, ProductCount, ProductDescription, ProductId, ProductName, ProductPrice,
        ProductSku, ProductUnits, ProductUrl,
    };
    use crate::forms::categories::SetProductCategoryFormPayload;
    use crate::repository::test::TestRepository;
    use chrono::DateTime;
    use pushkind_common::zmq::{SendFuture, ZmqSenderError, ZmqSenderTrait};

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
            associated_category: None,
            units: None,
            price: ProductPrice::new(1.0).unwrap(),
            amount: None,
            description: None,
            url: Some(ProductUrl::new("http://example.com/p").unwrap()),
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

    fn sample_benchmark() -> Benchmark {
        Benchmark {
            id: BenchmarkId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: BenchmarkName::new("benchmark").unwrap(),
            sku: BenchmarkSku::new("SKU1").unwrap(),
            category: CategoryName::new("cat").unwrap(),
            units: ProductUnits::new("pcs").unwrap(),
            price: ProductPrice::new(1.0).unwrap(),
            amount: ProductAmount::new(1.0).unwrap(),
            description: ProductDescription::new("desc").unwrap(),
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            embedding: None,
            processing: false,
            num_products: ProductCount::new(0).unwrap(),
        }
    }

    struct NoopSender;

    impl ZmqSenderTrait for NoopSender {
        fn send_bytes<'a>(&'a self, _bytes: Vec<u8>) -> SendFuture<'a> {
            Box::pin(async { Ok(()) })
        }

        fn try_send_bytes(&self, _bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
            Ok(())
        }

        fn send_multipart<'a>(&'a self, _frames: Vec<Vec<u8>>) -> SendFuture<'a> {
            Box::pin(async { Ok(()) })
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

    #[test]
    fn category_match_is_available_without_active_processing() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![], vec![sample_benchmark()]);
        let user = sample_user();

        assert!(can_match_product_categories(&user, &repo).unwrap());
    }

    #[test]
    fn category_match_is_unavailable_when_crawler_is_processing() {
        let mut crawler = sample_crawler();
        crawler.processing = true;
        let repo = TestRepository::new(vec![crawler], vec![], vec![sample_benchmark()]);
        let user = sample_user();

        assert!(!can_match_product_categories(&user, &repo).unwrap());
    }

    #[test]
    fn category_match_is_unavailable_when_benchmark_is_processing() {
        let mut benchmark = sample_benchmark();
        benchmark.processing = true;
        let repo = TestRepository::new(vec![sample_crawler()], vec![], vec![benchmark]);
        let user = sample_user();

        assert!(!can_match_product_categories(&user, &repo).unwrap());
    }

    #[test]
    fn match_product_categories_returns_form_error_when_processing_is_active() {
        let mut benchmark = sample_benchmark();
        benchmark.processing = true;
        let repo = TestRepository::new(vec![sample_crawler()], vec![], vec![benchmark]);
        let user = sample_user();
        let sender = NoopSender;

        let result = actix_web::rt::System::new()
            .block_on(async { match_product_categories(&user, &repo, &sender).await });

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }
}
