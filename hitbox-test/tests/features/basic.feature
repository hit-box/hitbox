Feature: Cache policy feature

  Scenario: first test scenario
    Given hitbox with policy
      ```yaml
      !Enabled
          ttl: 42
          stale: 43
      ```
    Given request predicates
      ```yaml
      And:
        - Method: GET
        - Path: /v1/authors/{author_id}/books
      ```
    Given response predicates
      ```yaml
      And:
        - Status: 200
      ```
    Given key extractors
      ```yaml
      - Method:
      - Path: /v1/authors/{author_id}/books
      - Query: page
      - Query: per_page
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books
      [Query]
      page: 1
      per_page: 20
      ```
    Then response status is 200
    And cache has records
      | per_page:20,page:1,author_id:robert-sheckley,method:GET | [{"id": "journey-beyond-tomorrow"}, {"id": "victim-prime"}] |
