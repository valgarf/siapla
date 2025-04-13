# \RegionalApi

All URIs are relative to *https://openholidaysapi.org*

Method | HTTP request | Description
------------- | ------------- | -------------
[**countries_get**](RegionalApi.md#countries_get) | **GET** /Countries | Returns a list of all supported countries
[**languages_get**](RegionalApi.md#languages_get) | **GET** /Languages | Returns a list of all used languages
[**subdivisions_get**](RegionalApi.md#subdivisions_get) | **GET** /Subdivisions | Returns a list of relevant subdivisions for a supported country (if any)



## countries_get

> Vec<models::CountryResponse> countries_get(language_iso_code)
Returns a list of all supported countries

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |

### Return type

[**Vec<models::CountryResponse>**](CountryResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## languages_get

> Vec<models::LanguageResponse> languages_get(language_iso_code)
Returns a list of all used languages

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |

### Return type

[**Vec<models::LanguageResponse>**](LanguageResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## subdivisions_get

> Vec<models::SubdivisionResponse> subdivisions_get(country_iso_code, language_iso_code)
Returns a list of relevant subdivisions for a supported country (if any)

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**country_iso_code** | **String** | ISO 3166-1 code of the country | [required] |
**language_iso_code** | Option<**String**> | ISO-639-1 code of a language or empty |  |

### Return type

[**Vec<models::SubdivisionResponse>**](SubdivisionResponse.md)

### Authorization

No authorization required

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json, text/json, text/plain, text/csv, application/problem+json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

