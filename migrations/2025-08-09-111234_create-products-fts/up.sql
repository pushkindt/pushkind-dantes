CREATE VIRTUAL TABLE products_fts USING fts5(
    name,
    sku,
    category,
    description,
    content='products',
    content_rowid='id',
    tokenize='unicode61'
);

INSERT INTO products_fts(products_fts) VALUES('rebuild');

CREATE TRIGGER products_ai AFTER INSERT ON products BEGIN
  INSERT INTO products_fts(rowid, name, sku, category, description) 
  VALUES (new.id, new.name, new.sku, new.category, new.description);
END;

CREATE TRIGGER products_ad AFTER DELETE ON products BEGIN
  INSERT INTO products_fts(products_fts, rowid, name, sku, category, description) 
  VALUES('delete', old.id, old.name, old.sku, old.category, old.description);
END;

CREATE TRIGGER products_au AFTER UPDATE ON products BEGIN
  INSERT INTO products_fts(products_fts, rowid, name, sku, category, description) 
  VALUES('delete', old.id, old.name, old.sku, old.category, old.description);
  INSERT INTO products_fts(rowid, name, sku, category, description) 
  VALUES (new.id, new.name, new.sku, new.category, new.description);
END;
