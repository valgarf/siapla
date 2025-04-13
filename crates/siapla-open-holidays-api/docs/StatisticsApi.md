# \StatisticsApi

All URIs are relative to *https://openholidaysapi.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**statistics_public_holidays_get**](StatisticsApi.md#statistics_public_holidays_get) | **GET** /Statistics/PublicHolidays | Returns statistical data about public holidays for a given country.
[**statistics_school_holidays_get**](StatisticsApi.md#statistics_school_holidays_get) | **GET** /Statistics/SchoolHolidays | Returns statistical data about school holidays for a given country



## statistics_public_holidays_get

> Vec<models::StatisticsResponse> statistics_public_holidays_get(country_iso_code, subdivision_code)
Returns statistical data about public holidays for a given country.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**country_iso_code** | **String** | ISO 3166-1 code of the country | [required] |
**subdivision_code** | Option<**String**> | Code of the subdivision or empty |  |

### Return type

[**Vec<models::StatisticsResponse>**](StatisticsResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## statistics_school_holidays_get

> Vec<models::StatisticsResponse> statistics_school_holidays_get(country_iso_code, subdivision_code)
Returns statistical data about school holidays for a given country

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**country_iso_code** | **String** | ISO 3166-1 code of the country | [required] |
**subdivision_code** | Option<**String**> | Code of the subdivision or empty |  |

### Return type

[**Vec<models::StatisticsResponse>**](StatisticsResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

