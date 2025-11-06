Feature: Request Body Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10
      ```

  @integration
  Scenario: Extract JSON field from request body for cache key
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body: '.title'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"title":"Test Book","description":"Test Description"}
      ```
    Then cache key exists
      ```
      .title: "Test Book"
      ```

  @integration
  Scenario: Extract nested JSON field
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body: '.user.email'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"user":{"email":"test@example.com","name":"John Doe"},"action":"update"}
      ```
    Then cache key exists
      ```
      .user.email: "test@example.com"
      ```

  @integration
  Scenario: Extract array element by index
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body: '.tags[0]'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"tags":["fiction","scifi","classic"],"title":"Book"}
      ```
    Then cache key exists
      ```
      .tags[0]: "fiction"
      ```

  @integration
  Scenario: Extract null value
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Method:
      - Body: '.metadata'
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"title":"Test","metadata":null}
      ```
    Then cache key exists
      ```
      method: "GET"
      .metadata: null
      ```
