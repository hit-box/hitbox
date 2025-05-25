Feature: Cache policy feature

  Scenario: first test scenario
    Given hitbox with policy 42
      # ```yaml
      # enabled: true
      # ```
    Given request predicate method=GET
    Given request predicate query=cache
    # When I feed the cat
    # Then the cat is not hungry
