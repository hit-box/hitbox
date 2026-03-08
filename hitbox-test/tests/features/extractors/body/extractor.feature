Feature: Request Body Cache Key Extractor

  Background:
    Given hitbox with policy
      ```yaml
      Enabled:
        ttl: 10s
      ```

  # Hash extraction
  @extractor @body @hash
  Scenario: Hash full body
    Given request predicates
      ```yaml
      - Method: POST
      ```
    And key extractors
      ```yaml
      - Body:
          transforms: [hash]
      ```
    When execute request
      ```hurl
      POST http://localhost/echo
      {"data":"some content to hash"}
      ```
    Then cache key exists
      | body | 79dccacb4939ab2f6ed060148f3eab67b75cb62d4cdc25853b973d29cb755bca |

  # Jq extraction - single object
  @extractor @body @jq
  Scenario: Jq extract object fields
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{user_id: .user.id, role: .role}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"user":{"id":123,"name":"John"},"role":"admin"}
      ```
    Then cache key exists
      | user_id | 123   |
      | role    | admin |

  @extractor @body @jq
  Scenario: Jq extract array of objects
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "[.items[] | {id, name}]"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"items":[{"id":1,"name":"a","extra":"x"},{"id":2,"name":"b","extra":"y"}]}
      ```
    Then cache key exists
      | id   | 1 |
      | name | a |
      | id   | 2 |
      | name | b |

  @extractor @body @jq
  Scenario: Jq extract array of primitives
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "[.tags[]]"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"tags":["rust","cache","http"]}
      ```
    Then cache key exists
      | body | rust  |
      | body | cache |
      | body | http  |

  @extractor @body @jq
  Scenario: Jq extract single primitive
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: ".user.id"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"user":{"id":456}}
      ```
    Then cache key exists
      | body | 456 |

  # Jq value postprocessing
  @extractor @body @jq @postprocess
  Scenario: Jq lowercase transformation
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{role: .role | ascii_downcase}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"role":"ADMIN"}
      ```
    Then cache key exists
      | role | admin |

  @extractor @body @jq @postprocess
  Scenario: Jq uppercase transformation
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{status: .status | ascii_upcase}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"status":"active"}
      ```
    Then cache key exists
      | status | ACTIVE |

  @extractor @body @jq @postprocess
  Scenario: Jq join array to string
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{tags: .tags | join(\",\")}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"tags":["rust","cache","http"]}
      ```
    Then cache key exists
      | tags | rust,cache,http |

  @extractor @body @jq @postprocess
  Scenario: Jq number to string conversion
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{version: \"v\" + (.version | tostring)}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"version":2}
      ```
    Then cache key exists
      | version | v2 |

  @extractor @body @jq @postprocess
  Scenario: Jq trim prefix
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{id: .id | ltrimstr(\"user-\")}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"id":"user-12345"}
      ```
    Then cache key exists
      | id | 12345 |

  @extractor @body @jq @postprocess
  Scenario: Jq trim suffix
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{file: .filename | rtrimstr(\".json\")}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"filename":"config.json"}
      ```
    Then cache key exists
      | file | config |

  @extractor @body @jq @postprocess
  Scenario: Jq sort and unique array values
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "[.roles | unique | sort | .[]]"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"roles":["user","admin","user","guest"]}
      ```
    Then cache key exists
      | body | admin |
      | body | guest |
      | body | user  |

  @extractor @body @jq @postprocess
  Scenario: Jq string split and select
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{domain: .email | split(\"@\") | .[1]}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"email":"user@example.com"}
      ```
    Then cache key exists
      | domain | example.com |

  @extractor @body @jq @postprocess
  Scenario: Jq conditional value transformation
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{tier: (if .premium then \"premium\" else \"free\" end)}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"premium":true,"name":"John"}
      ```
    Then cache key exists
      | tier | premium |

  # Jq hash function (custom hitbox extension)
  @extractor @body @jq @hash
  Scenario: Jq hash string value
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{token: .token | hash}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"token":"secret-value-123"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @extractor @body @jq @hash
  Scenario: Jq hash selective fields (some hashed, some not)
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{user_id: .user_id, token: .token | hash}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"user_id":"user-123","token":"secret-token"}
      ```
    Then cache key exists
      | user_id | user-123         |
      | token   | 930bbdc51b6aed5c2a5678fd6e28dee7a05e8a4b643cfc0b4427c3efb86c0d94 |

  @extractor @body @jq @hash
  Scenario: Jq hash with preprocessing (lowercase then hash)
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{email_hash: .email | ascii_downcase | hash}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"email":"User@Example.COM"}
      ```
    Then cache key exists
      | email_hash | b4c9a289323b21a01c3e940f150eb9b8c542587f1abfd8f0e1cc1ffc5e475514 |

  @extractor @body @jq @hash
  Scenario: Jq hash number value
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{id_hash: .id | hash}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"id":12345}
      ```
    Then cache key exists
      | id_hash | 5994471abb01112afcc18159f6cc74b4f511b99806da59b3caf5a9c173cacfc5 |

  @extractor @body @jq @hash
  Scenario: Jq hash object value
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          jq: "{config_hash: .config | hash}"
      ```
    When execute request
      ```hurl
      GET http://localhost/v1/authors/robert-sheckley/books/victim-prime
      {"config":{"debug":true,"level":5}}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  # Regex extraction - using JSON body with regex matching
  @extractor @body @regex
  Scenario: Regex extract named capture groups
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"user":"(?<user>[a-z]+)","token":"(?<token>[a-z0-9]+)"'
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"user":"john","token":"abc123"}
      ```
    Then cache key exists
      | user  | john   |
      | token | abc123 |

  @extractor @body @regex
  Scenario: Regex extract single group with key
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"session":"([a-z0-9]+)"'
          key: session
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"session":"xyz789"}
      ```
    Then cache key exists
      | session | xyz789 |

  @extractor @body @regex
  Scenario: Regex global matching
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"id":(?<id>[0-9]+)'
          global: true
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"items":[{"id":1},{"id":2},{"id":3}]}
      ```
    Then cache key exists
      | id | 1 |
      | id | 2 |
      | id | 3 |

  @extractor @body @regex @hash
  Scenario: Regex with per-key hash transform
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"token":"(?<token>[^"]+)"'
          transforms:
            token: hash
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"token":"super-secret-value"}
      ```
    Then response status is 200
    And response header "X-Cache-Status" is "MISS"
    And cache has 1 records

  @extractor @body @regex @hash
  Scenario: Regex with selective per-key transforms (some hashed, some not)
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"user":"(?<user>[^"]+)","token":"(?<token>[^"]+)"'
          transforms:
            token: hash
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"user":"john","token":"secret123"}
      ```
    Then cache key exists
      | user  | john             |
      | token | fcf730b6d95236ecd3c9fc2d92d7b6b2bb061514961aec041d6c7a7192f592e4 |

  @extractor @body @regex @hash
  Scenario: Regex with full body hash transform
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"session":"([a-z0-9]+)"'
          key: session
          transforms: [hash]
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"session":"mysecrettoken"}
      ```
    Then cache key exists
      | session | 7afb4c3889fd608668a69adf181c4218fb70bee515c2320bbc828839a3989fc2 |

  @extractor @body @regex @transforms
  Scenario: Regex with full body transform chain (lowercase then hash)
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"email":"([^"]+)"'
          key: email
          transforms: [lowercase, hash]
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"email":"User@Example.COM"}
      ```
    Then cache key exists
      | email | b4c9a289323b21a01c3e940f150eb9b8c542587f1abfd8f0e1cc1ffc5e475514 |

  @extractor @body @regex @transforms
  Scenario: Regex with per-key transform chain
    Given request predicates
      ```yaml
      - Method: GET
      ```
    And key extractors
      ```yaml
      - Body:
          regex: '"user":"(?<user>[^"]+)","email":"(?<email>[^"]+)"'
          transforms:
            user: uppercase
            email: [lowercase, hash]
      ```
    When execute request
      ```hurl
      GET http://localhost/echo
      {"user":"john","email":"John@Example.COM"}
      ```
    Then cache key exists
      | user  | JOHN             |
      | email | 855f96e983f1f8e8be944692b6f719fd54329826cb62e98015efee8e2e071dd4 |
