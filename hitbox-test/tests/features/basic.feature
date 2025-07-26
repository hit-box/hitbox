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
      ```
    When execute request
      ```hurl
			GET http://localhost/v1/authors/robert-shekli/books
      [Query]
			page: 1
			per_page: 20
      ```
    Then response status is 200
    And cache has records
      | author_id:robert-shekli,method:GET | Hello, robert-shekli |
