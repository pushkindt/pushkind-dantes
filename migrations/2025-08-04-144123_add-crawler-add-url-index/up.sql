-- Your SQL goes here
INSERT INTO crawlers (id, hub_id, name, url, selector)
VALUES (2, 1, 'rusteaco', 'https://shop.rusteaco.ru', 'rusteaco');

CREATE UNIQUE INDEX idx_crawler_id_product_url ON products(crawler_id, url);
