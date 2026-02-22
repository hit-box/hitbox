Feature: Selective Cache FSM Behavior

  Background:
    Given upstream response delay is 100ms
    And concurrency control is disabled

  # =============================================================================
  # Single Config
  # =============================================================================

  @selective @single-config @miss
  Scenario: Single config matches - cache miss
    Given 1 selective configs
    And config 1 request is cacheable
    And config 1 response is cacheable
    And cache is empty
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And all responses should equal 100
    And cache should contain value 100
    And FSM states should be:
      | CheckPredicate {selective.config_index = 0} |
      | ExtractKey {selective.config_index = 0}     |
      | PollCache {concurrency.decision = disabled} |
      | PollUpstream                                |
      | CheckResponseCachePolicy                    |
      | UpdateCache                                 |
      | Response                                    |

  @selective @single-config @hit
  Scenario: Single config matches - cache hit
    Given 1 selective configs
    And config 1 request is cacheable
    And cache contains fresh value 42
    When 1 selective request is made with value 100
    Then upstream should not be called
    And all responses should equal 42
    And FSM states should be:
      | CheckPredicate {selective.config_index = 0} |
      | ExtractKey {selective.config_index = 0}     |
      | PollCache                                   |
      | ConvertResponse                             |
      | Response                                    |

  @selective @single-config @passthrough
  Scenario: Single config non-cacheable - passthrough
    Given 1 selective configs
    And config 1 request is non-cacheable
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And all responses should equal 100
    And cache should be empty
    And FSM states should be:
      | CheckPredicate {selective.config_index = 0} |
      | Passthrough                                 |

  # =============================================================================
  # Multi-Config Routing
  # =============================================================================

  @selective @multi-config
  Scenario: First of two configs matches
    Given 2 selective configs
    And config 1 request is cacheable
    And config 1 response is cacheable
    And config 2 request is cacheable
    And cache is empty
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And all responses should equal 100
    And FSM states should be:
      | CheckPredicate {selective.config_index = 0} |
      | ExtractKey {selective.config_index = 0}     |
      | PollCache {concurrency.decision = disabled} |
      | PollUpstream                                |
      | CheckResponseCachePolicy                    |
      | UpdateCache                                 |
      | Response                                    |

  @selective @multi-config
  Scenario: Second config matches after first rejects
    Given 2 selective configs
    And config 1 request is non-cacheable
    And config 2 request is cacheable
    And config 2 response is cacheable
    And cache is empty
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And cache should contain value 100
    And FSM states should be:
      | CheckPredicate {selective.config_index = 0} |
      | CheckPredicate {selective.config_index = 1} |
      | ExtractKey {selective.config_index = 1}     |
      | PollCache {concurrency.decision = disabled} |
      | PollUpstream                                |
      | CheckResponseCachePolicy                    |
      | UpdateCache                                 |
      | Response                                    |

  @selective @multi-config @passthrough
  Scenario: No config matches - passthrough
    Given 2 selective configs
    And config 1 request is non-cacheable
    And config 2 request is non-cacheable
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And all responses should equal 100
    And cache should be empty
    And FSM states should be:
      | CheckPredicate {selective.config_index = 0} |
      | CheckPredicate {selective.config_index = 1} |
      | Passthrough                                 |

  # =============================================================================
  # Disabled Config Handling
  # =============================================================================

  @selective @disabled
  Scenario: Disabled config is skipped
    Given 2 selective configs
    And config 1 cache policy is "Disabled"
    And config 2 request is cacheable
    And config 2 response is cacheable
    And cache is empty
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And cache should contain value 100
    And FSM states should be:
      | CheckPredicate {selective.config_index = 1} |
      | ExtractKey {selective.config_index = 1}     |
      | PollCache {concurrency.decision = disabled} |
      | PollUpstream                                |
      | CheckResponseCachePolicy                    |
      | UpdateCache                                 |
      | Response                                    |

  @selective @disabled @passthrough
  Scenario: All configs disabled - passthrough
    Given 2 selective configs
    And config 1 cache policy is "Disabled"
    And config 2 cache policy is "Disabled"
    When 1 selective request is made with value 100
    Then upstream should be called 1 time
    And all responses should equal 100
    And cache should be empty
